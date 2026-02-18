#include <algorithm>
#include <functional>
#include <limits>
#include <map>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

#include <diagnostic_updater/diagnostic_updater.hpp>
#include <lifecycle_msgs/msg/state.hpp>
#include <rclcpp/generic_subscription.hpp>
#include <rclcpp/serialized_message.hpp>
#include <rclcpp/typesupport_helpers.hpp>
#include <rclcpp_components/register_node_macro.hpp>
#include <rclcpp_lifecycle/lifecycle_node.hpp>
#include <rcpputils/shared_library.hpp>
#include <rmw/rmw.h>
#include <rosidl_typesupport_introspection_cpp/message_introspection.hpp>

#include <sentil/sentil.hpp>

#include "sentil_ros/field_extractor.hpp"
#include "sentil_ros/msg/probability.hpp"
#include "sentil_ros/msg/robustness.hpp"
#include "sentil_ros/srv/get_spec_info.hpp"

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

TypeSupport load_type_support(const std::string & type_name)
{
  TypeSupport ts;
  ts.introspection_lib =
    rclcpp::get_typesupport_library(type_name, "rosidl_typesupport_introspection_cpp");
  ts.introspection = rclcpp::get_typesupport_handle(
    type_name, "rosidl_typesupport_introspection_cpp", *ts.introspection_lib);
  ts.rmw_lib = rclcpp::get_typesupport_library(type_name, "rosidl_typesupport_cpp");
  ts.rmw = rclcpp::get_typesupport_handle(type_name, "rosidl_typesupport_cpp", *ts.rmw_lib);
  return ts;
}

sentil::NoiseModel noise_from_params(
  rclcpp_lifecycle::LifecycleNode * node, const std::string & base, const std::string & family)
{
  const auto d = [&](const std::string & key, double fallback) {
    const std::string name = base + "." + key;
    return node->has_parameter(name) ? node->get_parameter(name).as_double()
                                     : node->declare_parameter(name, fallback);
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
  if (family == "beta") {
    return sentil::NoiseModel::beta(d("alpha", 1.0), d("beta", 1.0));
  }
  throw std::runtime_error(
    "noise.type '" + family + "' is not recognized; expected gaussian, uniform, log_normal, "
    "exponential, gamma, beta, or none");
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
    std::string name;
    std::string topic;
    std::string field;
    TypeSupport type_support;
    std::vector<uint8_t> scratch;  // reused message storage, sized once at configure
    rclcpp::GenericSubscription::SharedPtr subscription;
  };

  template<typename T>
  T declare_once(const std::string & name, const T & fallback)
  {
    return has_parameter(name) ? get_parameter(name).get_value<T>() : declare_parameter<T>(name, fallback);
  }

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

    for (const auto & [var, topic] : var_topic) {
      Binding binding;
      binding.name = var;
      binding.topic = topic;
      binding.field = var_field.at(var);
      const std::string type_name = resolve_type(topic, var_type.at(var));
      binding.type_support = load_type_support(type_name);
      const auto * members = static_cast<const rosidl_typesupport_introspection_cpp::MessageMembers *>(
        binding.type_support.introspection->data);
      binding.scratch.resize(members->size_of_);
      const rclcpp::QoS qos = match_qos(topic);
      Binding & stored = bindings_.emplace(var, std::move(binding)).first->second;
      stored.subscription = create_generic_subscription(
        topic, type_name, qos,
        [this, ptr = &stored](std::shared_ptr<rclcpp::SerializedMessage> msg) {
          on_sample(*ptr, std::move(msg));
        });
    }

    diagnostics_ = std::make_unique<diagnostic_updater::Updater>(this);
    diagnostics_->setHardwareID("sentil");
    for (const auto & id : labels_) {
      diagnostics_->add(id, [this, id](diagnostic_updater::DiagnosticStatusWrapper & status) {
        report_diagnostic(id, status);
      });
    }

    spec_info_service_ = create_service<srv::GetSpecInfo>(
      "~/get_spec_info",
      [this](
        const std::shared_ptr<srv::GetSpecInfo::Request> request,
        std::shared_ptr<srv::GetSpecInfo::Response> response) {
        handle_get_spec_info(request, response);
      });
  }

  void handle_get_spec_info(
    const std::shared_ptr<srv::GetSpecInfo::Request> request,
    std::shared_ptr<srv::GetSpecInfo::Response> response)
  {
    try {
      auto builder = request->spec_file.empty()
        ? sentil::SpecBuilder(request->spec_name)
        : sentil::SpecBuilder::from_file(request->spec_file);
      bool built = false;
      try {
        response->deterministic_formula = builder.build_deterministic();
        built = true;
      } catch (const std::exception &) {
      }
      try {
        response->probabilistic_formula = builder.build_probabilistic();
        built = true;
      } catch (const std::exception &) {
      }
      response->parameters_json = builder.parameters_json();
      for (const auto & variant : builder.available_variants()) {
        response->available_variants.push_back(variant);
      }
      response->success = built;
      if (!built) {
        response->error_message = "the spec carries neither a deterministic nor a probabilistic formula";
      }
    } catch (const std::exception & e) {
      response->success = false;
      response->error_message = e.what();
    }
  }

  void configure_formula(
    const std::string & id, std::map<std::string, std::string> & var_topic,
    std::map<std::string, std::string> & var_field, std::map<std::string, std::string> & var_type)
  {
    const std::string base = "formulas." + id;
    const std::string spec_name = declare_once<std::string>(base + ".spec", "");
    const std::string raw = declare_once<std::string>(base + ".formula", "");
    const std::string method = declare_once<std::string>(base + ".verification.method", "automatic");
    // A ROS parameter cannot be both a string array and a nested namespace.
    const auto variables = declare_once<std::vector<std::string>>(base + ".signal_names", {});

    const std::string var_prefix = base + ".variables.";
    for (const auto & entry : get_node_parameters_interface()->get_parameter_overrides()) {
      const std::string & key = entry.first;
      if (key.compare(0, var_prefix.size(), var_prefix) != 0) {
        continue;
      }
      const std::string var = key.substr(var_prefix.size(), key.find('.', var_prefix.size()) - var_prefix.size());
      if (std::find(variables.begin(), variables.end(), var) == variables.end()) {
        throw std::runtime_error(
          "variable '" + var + "' of formula '" + id + "' is configured under variables but missing "
          "from signal_names; add it to formulas." + id + ".signal_names");
      }
    }

    std::string formula_text = raw;
    sentil::LiftingRegistry lifting;
    if (method == "sprt") {
      throw std::runtime_error(
        "formula '" + id + "': verification.method 'sprt' is not available online; the streaming "
        "monitor estimates the probability continuously, so use 'smc' or leave the method automatic");
    }
    if (method != "robustness" && method != "smc" && method != "automatic") {
      throw std::runtime_error(
        "formula '" + id + "': verification.method '" + method + "' is not recognized; expected "
        "robustness, smc, or automatic");
    }
    bool probabilistic = method == "smc";
    if (!spec_name.empty()) {
      auto builder = sentil::SpecBuilder(spec_name);
      const std::string variant = declare_once<std::string>(base + ".variant", "");
      if (!variant.empty()) {
        builder = std::move(builder).with_variant(variant);
      }
      const auto names = declare_once<std::vector<std::string>>(base + ".spec_params.names", {});
      const auto values = declare_once<std::vector<double>>(base + ".spec_params.values", {});
      if (names.size() != values.size()) {
        throw std::runtime_error("formula '" + id + "': spec_params names and values differ in length");
      }
      for (size_t i = 0; i < names.size(); ++i) {
        builder = std::move(builder).with_param(names[i], values[i]);
      }
      lifting = builder.build_lifting_registry();
      if (method != "robustness") {
        probabilistic = probabilistic || !lifting.variables().empty();
      }
      formula_text = probabilistic ? builder.build_probabilistic() : builder.build_deterministic();
    }
    for (const auto & var : variables) {
      const std::string vbase = base + ".variables." + var;
      var_topic[var] = declare_once<std::string>(vbase + ".topic", "");
      var_field[var] = declare_once<std::string>(vbase + ".field", "");
      var_type[var] = declare_once<std::string>(vbase + ".type", "");
      const std::string family = declare_once<std::string>(vbase + ".noise.type", "none");
      if (family != "none" && method != "robustness") {
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
      prob_samples_[id] = config.samples;
      prob_confidence_[id] = config.confidence;
      prob_publishers_[id] = create_publisher<msg::Probability>("~/" + id + "/probability", 10);
    } else {
      monitor_->add(id, formula);
    }
    labels_.push_back(id);
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

  void on_sample(Binding & binding, std::shared_ptr<rclcpp::SerializedMessage> msg)
  {
    if (!active_) {
      return;
    }
    const auto * members = static_cast<const rosidl_typesupport_introspection_cpp::MessageMembers *>(
      binding.type_support.introspection->data);
    uint8_t * buffer = binding.scratch.data();
    members->init_function(buffer, rosidl_runtime_cpp::MessageInitialization::ZERO);
    double value = 0.0;
    bool ok = true;
    try {
      if (rmw_deserialize(
          &msg->get_rcl_serialized_message(), binding.type_support.rmw, buffer) != RMW_RET_OK)
      {
        throw std::runtime_error("deserialize failed");
      }
      value = introspection::extract_double_from_field(
        buffer, binding.type_support.introspection, binding.field);
    } catch (const std::exception & e) {
      RCLCPP_WARN_THROTTLE(get_logger(), *get_clock(), 1000, "sample on %s: %s", binding.name.c_str(), e.what());
      ok = false;
    }
    members->fini_function(buffer);
    if (!ok) {
      return;
    }

    state_[binding.name] = value;
    if (state_.size() < bindings_.size()) {
      return;  // hold until every distinct variable has been seen at least once
    }
    const double stamp = now().seconds();
    if (stamp <= last_stamp_) {
      // The streaming monitor needs strictly increasing time; under a stalled or rewound
      // clock skip the sample rather than feed it a non-monotonic stamp.
      RCLCPP_WARN_THROTTLE(get_logger(), *get_clock(), 1000, "non-monotonic stamp %.6f, skipping", stamp);
      return;
    }
    last_stamp_ = stamp;
    try {
      const auto verdicts = monitor_->update(stamp, state_);
      for (const auto & [id, robustness] : verdicts) {
        last_verdict_[id] = robustness;
        publish(id, robustness);
        publish_probability(id);
      }
    } catch (const std::exception & e) {
      RCLCPP_WARN_THROTTLE(get_logger(), *get_clock(), 1000, "update failed: %s", e.what());
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

  /// Publish the running satisfaction probability of a P~p formula, the estimate the
  /// streaming monitor maintains over its lifted ensemble. Deterministic formulas have
  /// no probability publisher and are skipped.
  void publish_probability(const std::string & id)
  {
    const auto pub = prob_publishers_.find(id);
    if (pub == prob_publishers_.end()) {
      return;
    }
    const auto estimate = monitor_->probability(id);
    if (!estimate) {
      return;
    }
    const std::uint64_t samples = prob_samples_.at(id);
    msg::Probability out;
    out.header.stamp = now();
    out.formula_id = id;
    out.estimate = *estimate;
    out.samples = samples;
    out.satisfactions = static_cast<std::uint64_t>(std::llround(*estimate * static_cast<double>(samples)));
    // The running estimate is the satisfying fraction over the lifted ensemble, so a
    // Wilson interval over those counts is the same band the offline SMC reports.
    const auto interval = sentil::stats::wilson_interval(out.satisfactions, samples, prob_confidence_.at(id));
    out.ci_lower = interval.lower;
    out.ci_upper = interval.upper;
    out.ci_confidence = interval.level;
    pub->second->publish(out);
  }

  void report_diagnostic(const std::string & id, diagnostic_updater::DiagnosticStatusWrapper & status)
  {
    using diagnostic_msgs::msg::DiagnosticStatus;
    const auto it = last_verdict_.find(id);
    if (it == last_verdict_.end()) {
      std::string waiting;
      for (const auto & entry : bindings_) {
        if (state_.find(entry.first) == state_.end()) {
          waiting += waiting.empty() ? entry.first : ", " + entry.first;
        }
      }
      status.summary(DiagnosticStatus::STALE, waiting.empty() ? "no data yet" : "waiting for: " + waiting);
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
    prob_publishers_.clear();
    prob_samples_.clear();
    prob_confidence_.clear();
    diagnostics_.reset();
    spec_info_service_.reset();
    monitor_.reset();
    labels_.clear();
    state_.clear();
    last_verdict_.clear();
    last_stamp_ = -std::numeric_limits<double>::infinity();
  }

  // Callbacks all run on the node's default mutually exclusive group.
  std::unique_ptr<sentil::MultiMonitor> monitor_;
  std::map<std::string, Binding> bindings_;
  std::map<std::string, rclcpp_lifecycle::LifecyclePublisher<msg::Robustness>::SharedPtr> publishers_;
  std::map<std::string, rclcpp_lifecycle::LifecyclePublisher<msg::Probability>::SharedPtr> prob_publishers_;
  std::map<std::string, std::uint64_t> prob_samples_;
  std::map<std::string, double> prob_confidence_;
  std::unique_ptr<diagnostic_updater::Updater> diagnostics_;
  rclcpp::Service<srv::GetSpecInfo>::SharedPtr spec_info_service_;
  std::vector<std::string> labels_;
  std::map<std::string, double> state_;
  std::map<std::string, sentil::Robustness> last_verdict_;
  double last_stamp_ = -std::numeric_limits<double>::infinity();
  bool active_ = false;
};

}  // namespace sentil_ros

RCLCPP_COMPONENTS_REGISTER_NODE(sentil_ros::MonitorNode)