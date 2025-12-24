#include <dlfcn.h>

#include <functional>
#include <map>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

#include <ament_index_cpp/get_package_share_directory.hpp>
#include <diagnostic_updater/diagnostic_updater.hpp>
#include <lifecycle_msgs/msg/state.hpp>
#include <rclcpp/generic_subscription.hpp>
#include <rclcpp/serialized_message.hpp>
#include <rclcpp_components/register_node_macro.hpp>
#include <rclcpp_lifecycle/lifecycle_node.hpp>
#include <rcpputils/shared_library.hpp>
#include <rmw/rmw.h>
#include <rosidl_typesupport_introspection_cpp/message_introspection.hpp>

#include <sentil/sentil.hpp>

#include "sentil_ros/field_extractor.hpp"
#include "sentil_ros/msg/robustness.hpp"

namespace sentil_ros
{

using CallbackReturn = rclcpp_lifecycle::node_interfaces::LifecycleNodeInterface::CallbackReturn;

namespace
{

// The handles stay valid only while their libraries are held open.
struct TypeSupport
{
  std::shared_ptr<rcpputils::SharedLibrary> introspection_lib;
  std::shared_ptr<rcpputils::SharedLibrary> rmw_lib;
  const rosidl_message_type_support_t * introspection = nullptr;
  const rosidl_message_type_support_t * rmw = nullptr;
};

std::pair<std::string, std::string> split_type(const std::string & type_name)
{
  const auto slash = type_name.find('/');
  const auto last = type_name.rfind('/');
  if (slash == std::string::npos || last == std::string::npos) {
    throw std::runtime_error("type '" + type_name + "' is not pkg/msg/Name");
  }
  return {type_name.substr(0, slash), type_name.substr(last + 1)};
}

TypeSupport load_type_support(const std::string & type_name)
{
  const auto [package, name] = split_type(type_name);
  const std::string share = ament_index_cpp::get_package_share_directory(package);
  const std::string lib_dir = share + "/../../lib";

  TypeSupport ts;
  const std::string intro_path =
    lib_dir + "/lib" + package + "__rosidl_typesupport_introspection_cpp.so";
  const std::string rmw_path = lib_dir + "/lib" + package + "__rosidl_typesupport_cpp.so";
  ts.introspection_lib = std::make_shared<rcpputils::SharedLibrary>(intro_path);
  ts.rmw_lib = std::make_shared<rcpputils::SharedLibrary>(rmw_path);

  const std::string intro_symbol =
    "rosidl_typesupport_introspection_cpp__get_message_type_support_handle__" + package +
    "__msg__" + name;
  const std::string rmw_symbol =
    "rosidl_typesupport_cpp__get_message_type_support_handle__" + package + "__msg__" + name;
  using HandleFn = const rosidl_message_type_support_t * (*)();
  ts.introspection = reinterpret_cast<HandleFn>(ts.introspection_lib->get_symbol(intro_symbol))();
  ts.rmw = reinterpret_cast<HandleFn>(ts.rmw_lib->get_symbol(rmw_symbol))();
  return ts;
}

/// Builds a noise model from the `<base>` parameter namespace, defaulting to a Dirac
/// (no noise) when the family is unknown.
sentil::NoiseModel noise_from_params(
  rclcpp_lifecycle::LifecycleNode * node, const std::string & base, const std::string & family)
{
  const auto d = [&](const std::string & key, double fallback) {
    return node->declare_parameter(base + "." + key, fallback);
  };
  if (family == "gaussian") {
    return sentil::NoiseModel::gaussian(d("mean", 0.0), d("std_dev", 1.0));
  }
  if (family == "uniform") {
    return sentil::NoiseModel::uniform(d("low", -1.0), d("high", 1.0));
  }
  if (family == "log_normal") {
    return sentil::NoiseModel::log_normal(d("mu", 0.0), d("sigma", 1.0));
  }
  if (family == "exponential") {
    return sentil::NoiseModel::exponential(d("rate", 1.0));
  }
  if (family == "gamma") {
    return sentil::NoiseModel::gamma(d("shape", 1.0), d("scale", 1.0));
  }
  if (family == "uniform") {
    return sentil::NoiseModel::uniform(d("low", -1.0), d("high", 1.0));
  }
  return sentil::NoiseModel::dirac(d("value", 0.0));
}

}  // namespace

class MonitorNode : public rclcpp_lifecycle::LifecycleNode
{
public:
  explicit MonitorNode(const rclcpp::NodeOptions & options)
  : rclcpp_lifecycle::LifecycleNode("sentil_monitor", options)
  {
    auto desc = rcl_interfaces::msg::ParameterDescriptor();
    desc.description = "Identifiers of the formulas to monitor; each is configured under formulas.<id>.";
    declare_parameter<std::vector<std::string>>("formulas", std::vector<std::string>{}, desc);
  }

  CallbackReturn on_configure(const rclcpp_lifecycle::State &) override
  {
    try {
      build();
    } catch (const std::exception & e) {
      RCLCPP_ERROR(get_logger(), "configuration failed: %s", e.what());
      teardown();
      return CallbackReturn::FAILURE;
    }
    RCLCPP_INFO(get_logger(), "configured %zu formula(s)", labels_.size());
    return CallbackReturn::SUCCESS;
  }

  CallbackReturn on_activate(const rclcpp_lifecycle::State & state) override
  {
    LifecycleNode::on_activate(state);
    active_ = true;
    return CallbackReturn::SUCCESS;
  }

  CallbackReturn on_deactivate(const rclcpp_lifecycle::State & state) override
  {
    active_ = false;
    LifecycleNode::on_deactivate(state);
    return CallbackReturn::SUCCESS;
  }

  CallbackReturn on_cleanup(const rclcpp_lifecycle::State &) override
  {
    teardown();
    return CallbackReturn::SUCCESS;
  }

  CallbackReturn on_shutdown(const rclcpp_lifecycle::State &) override
  {
    teardown();
    return CallbackReturn::SUCCESS;
  }

private:
  struct Binding
  {
    std::string topic;
    std::string field;
    TypeSupport type_support;
    rclcpp::GenericSubscription::SharedPtr subscription;
  };

  void build()
  {
    monitor_ = std::make_unique<sentil::MultiMonitor>();
    const auto formulas = get_parameter("formulas").as_string_array();
    if (formulas.empty()) {
      throw std::runtime_error("no formulas configured; set the 'formulas' parameter");
    }

    std::map<std::string, std::string> var_topic;
    std::map<std::string, std::string> var_field;
    std::map<std::string, std::string> var_type;
    for (const auto & id : formulas) {
      configure_formula(id, var_topic, var_field, var_type);
    }

    // One subscription per distinct variable, QoS-matched to the publisher.
    for (const auto & [var, topic] : var_topic) {
      Binding binding;
      binding.topic = topic;
      binding.field = var_field.at(var);
      const std::string type_name = resolve_type(topic, var_type[var]);
      binding.type_support = load_type_support(type_name);
      const rclcpp::QoS qos = match_qos(topic);
      const std::string captured = var;
      binding.subscription = create_generic_subscription(
        topic, type_name, qos,
        [this, captured](std::shared_ptr<rclcpp::SerializedMessage> msg) {
          on_sample(captured, std::move(msg));
        });
      bindings_.emplace(var, std::move(binding));
    }

    diagnostics_ = std::make_unique<diagnostic_updater::Updater>(this);
    diagnostics_->setHardwareID("sentil");
    for (const auto & id : labels_) {
      diagnostics_->add(id, [this, id](diagnostic_updater::DiagnosticStatusWrapper & status) {
        report_diagnostic(id, status);
      });
    }
  }

  void configure_formula(
    const std::string & id, std::map<std::string, std::string> & var_topic,
    std::map<std::string, std::string> & var_field, std::map<std::string, std::string> & var_type)
  {
    const std::string base = "formulas." + id;
    const std::string spec_name = declare_parameter<std::string>(base + ".spec", "");
    const std::string raw = declare_parameter<std::string>(base + ".formula", "");
    const std::string method = declare_parameter<std::string>(base + ".verification.method", "automatic");
    const auto variables = declare_parameter<std::vector<std::string>>(base + ".variables", {});

    std::string formula_text = raw;
    sentil::LiftingRegistry lifting;
    bool probabilistic = method == "smc" || method == "sprt";
    if (!spec_name.empty()) {
      auto builder = sentil::SpecBuilder(spec_name);
      const std::string variant = declare_parameter<std::string>(base + ".variant", "");
      if (!variant.empty()) {
        builder = std::move(builder).with_variant(variant);
      }
      lifting = builder.build_lifting_registry();
      probabilistic = probabilistic || !lifting.variables().empty();
      formula_text = probabilistic ? builder.build_probabilistic() : builder.build_deterministic();
    }
    for (const auto & var : variables) {
      const std::string vbase = base + ".variables." + var;
      var_topic[var] = declare_parameter<std::string>(vbase + ".topic", "");
      var_field[var] = declare_parameter<std::string>(vbase + ".field", "");
      var_type[var] = declare_parameter<std::string>(vbase + ".type", "");
      const std::string family = declare_parameter<std::string>(vbase + ".noise.type", "none");
      if (family != "none") {
        probabilistic = true;
        lifting.register_noise(var, noise_from_params(this, vbase + ".noise", family));
      }
      if (var_topic[var].empty() || var_field[var].empty()) {
        throw std::runtime_error("variable '" + var + "' of formula '" + id + "' needs a topic and field");
      }
    }

    auto formula = sentil::Formula::parse(formula_text);
    if (probabilistic) {
      sentil::SmcConfig config;
      config.samples = static_cast<std::uint64_t>(
        declare_parameter<int>(base + ".config.particles", 1000));
      config.confidence = declare_parameter<double>(base + ".config.confidence", 0.95);
      monitor_->add_probabilistic(id, formula, lifting, config);
    } else {
      monitor_->add(id, formula);
    }
    labels_.push_back(id);
    expected_vars_ += variables.size();
    publishers_[id] = create_publisher<msg::Robustness>("~/" + id + "/robustness", 10);
  }

  std::string resolve_type(const std::string & topic, const std::string & configured)
  {
    if (!configured.empty()) {
      return configured;
    }
    const auto types = get_topic_names_and_types();
    const auto it = types.find(topic);
    if (it == types.end() || it->second.empty()) {
      throw std::runtime_error("no publisher and no type configured for topic " + topic);
    }
    return it->second.front();
  }

  // A reliability or durability mismatch drops the link silently.
  rclcpp::QoS match_qos(const std::string & topic)
  {
    rclcpp::QoS qos = rclcpp::SensorDataQoS().keep_last(10);
    const auto publishers = get_publishers_info_by_topic(topic);
    if (!publishers.empty()) {
      const auto & profile = publishers.front().qos_profile();
      qos.reliability(profile.reliability());
      qos.durability(profile.durability());
    }
    return qos;
  }

  void on_sample(const std::string & variable, std::shared_ptr<rclcpp::SerializedMessage> msg)
  {
    if (!active_) {
      return;
    }
    const Binding & binding = bindings_.at(variable);
    const auto * members = static_cast<const rosidl_typesupport_introspection_cpp::MessageMembers *>(
      binding.type_support.introspection->data);
    std::vector<uint8_t> buffer(members->size_of_);
    members->init_function(buffer.data(), rosidl_runtime_cpp::MessageInitialization::ZERO);
    double value = 0.0;
    bool ok = true;
    try {
      if (rmw_deserialize(
          &msg->get_rcl_serialized_message(), binding.type_support.rmw, buffer.data()) != RMW_RET_OK)
      {
        throw std::runtime_error("deserialize failed");
      }
      value = introspection::extract_double_from_field(
        buffer.data(), binding.type_support.introspection, binding.field);
    } catch (const std::exception & e) {
      RCLCPP_WARN_THROTTLE(get_logger(), *get_clock(), 1000, "sample on %s: %s", variable.c_str(), e.what());
      ok = false;
    }
    members->fini_function(buffer.data());
    if (!ok) {
      return;
    }

    state_[variable] = value;
    if (state_.size() < expected_vars_) {
      return;  // hold until every variable has been seen at least once
    }
    const double stamp = now().seconds();
    const auto verdicts = monitor_->update(stamp, state_);
    for (const auto & [id, robustness] : verdicts) {
      last_verdict_[id] = robustness;
      publish(id, robustness);
    }
  }

  void publish(const std::string & id, const sentil::Robustness & robustness)
  {
    msg::Robustness out;
    out.header.stamp = now();
    out.formula_id = id;
    out.robustness = robustness.value;
    out.is_concrete = robustness.resolved;
    out.robustness_min = robustness.lower;
    out.robustness_max = robustness.upper;
    publishers_.at(id)->publish(out);
  }

  void report_diagnostic(const std::string & id, diagnostic_updater::DiagnosticStatusWrapper & status)
  {
    using diagnostic_msgs::msg::DiagnosticStatus;
    const auto it = last_verdict_.find(id);
    if (it == last_verdict_.end()) {
      status.summary(DiagnosticStatus::STALE, "no data yet");
      return;
    }
    const sentil::Robustness & r = it->second;
    if (!r.resolved) {
      status.summary(DiagnosticStatus::WARN, "undecided");
    } else if (r.satisfied) {
      status.summary(DiagnosticStatus::OK, "satisfied");
    } else {
      status.summary(DiagnosticStatus::ERROR, "violated");
    }
    status.add("robustness", r.value);
  }

  void teardown()
  {
    bindings_.clear();
    publishers_.clear();
    diagnostics_.reset();
    monitor_.reset();
    labels_.clear();
    state_.clear();
    last_verdict_.clear();
    expected_vars_ = 0;
  }

  std::unique_ptr<sentil::MultiMonitor> monitor_;
  std::map<std::string, Binding> bindings_;
  std::map<std::string, rclcpp_lifecycle::LifecyclePublisher<msg::Robustness>::SharedPtr> publishers_;
  std::unique_ptr<diagnostic_updater::Updater> diagnostics_;
  std::vector<std::string> labels_;
  std::map<std::string, double> state_;
  std::map<std::string, sentil::Robustness> last_verdict_;
  size_t expected_vars_ = 0;
  bool active_ = false;
};

}  // namespace sentil_ros

RCLCPP_COMPONENTS_REGISTER_NODE(sentil_ros::MonitorNode)