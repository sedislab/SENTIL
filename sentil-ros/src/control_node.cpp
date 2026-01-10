// The SENTIL control node is a managed (lifecycle) component that synthesizes control
// from a PrSTL specification and actuates it on ROS 2 topics. It exposes the whole
// synthesis subsystem:
//
//   receding_horizon  an online controller that, each step, plans over a short horizon
//                     and emits the first input within a hard deadline
//   open_loop         offline trajectory synthesis: an input sequence that satisfies
//                     the spec, published as a trajectory and stepped out in real time
//   safety_filter     a control-barrier-function shield that takes a nominal command
//                     and returns the closest input that respects the bounds and barriers
//
// The system model, spec, input bounds, and mode come from configuration. The current
// state arrives on a std_msgs/Float64MultiArray topic; the control command goes out as
// a sentil_ros/Control message and a plain Float64MultiArray.

#include <chrono>
#include <cmath>
#include <cstdint>
#include <limits>
#include <memory>
#include <mutex>
#include <stdexcept>
#include <string>
#include <vector>

#include <lifecycle_msgs/msg/state.hpp>
#include <rclcpp_components/register_node_macro.hpp>
#include <rclcpp_lifecycle/lifecycle_node.hpp>
#include <std_msgs/msg/float64_multi_array.hpp>

#include <sentil/sentil.hpp>

#include "sentil_ros/msg/control.hpp"

namespace sentil_ros
{

using CallbackReturn = rclcpp_lifecycle::node_interfaces::LifecycleNodeInterface::CallbackReturn;
using Float64MultiArray = std_msgs::msg::Float64MultiArray;

class ControlNode : public rclcpp_lifecycle::LifecycleNode
{
public:
  explicit ControlNode(const rclcpp::NodeOptions & options)
  : rclcpp_lifecycle::LifecycleNode("sentil_control", options)
  {
    declare_parameter<std::string>("mode", "receding_horizon");
    declare_parameter<std::string>("spec.formula", "");
    declare_parameter<std::string>("spec.name", "");
    declare_parameter<std::string>("spec.variant", "");
    declare_parameter<int>("model.state_dim", 0);
    declare_parameter<std::vector<double>>("model.a", {});
    declare_parameter<std::vector<double>>("model.b", {});
    declare_parameter<std::vector<double>>("model.x0", {});
    declare_parameter<std::vector<std::string>>("model.variables", {});
    declare_parameter<double>("model.dt", 0.1);
    declare_parameter<int>("model.horizon", 20);
    declare_parameter<int>("input_width", 1);
    declare_parameter<double>("budget_ms", 20.0);
    declare_parameter<std::vector<double>>("bounds.lower", {});
    declare_parameter<std::vector<double>>("bounds.upper", {});
    declare_parameter<std::string>("state_topic", "~/state");
    declare_parameter<std::string>("nominal_topic", "~/nominal");
    declare_parameter<std::string>("control_topic", "~/command");
    declare_parameter<double>("rate_hz", 10.0);
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
    RCLCPP_INFO(get_logger(), "configured a %s controller over %zu state(s), %zu input(s)",
                mode_.c_str(), state_dim_, input_width_);
    return CallbackReturn::SUCCESS;
  }

  CallbackReturn on_activate(const rclcpp_lifecycle::State & state) override
  {
    LifecycleNode::on_activate(state);
    active_ = true;
    if (mode_ == "open_loop") {
      step_ = 0;
      timer_ = create_wall_timer(period_, [this]() { emit_open_loop_step(); });
    }
    return CallbackReturn::SUCCESS;
  }

  CallbackReturn on_deactivate(const rclcpp_lifecycle::State & state) override
  {
    active_ = false;
    timer_.reset();
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
  std::vector<std::vector<double>> reshape(const std::vector<double> & flat, std::size_t rows,
                                           std::size_t cols, const std::string & what)
  {
    if (flat.size() != rows * cols) {
      throw std::runtime_error(what + " must have " + std::to_string(rows * cols) + " entries");
    }
    std::vector<std::vector<double>> out(rows, std::vector<double>(cols));
    for (std::size_t i = 0; i < rows; ++i) {
      for (std::size_t j = 0; j < cols; ++j) {
        out[i][j] = flat[i * cols + j];
      }
    }
    return out;
  }

  static std::vector<double> tile(const std::vector<double> & v, std::size_t n)
  {
    std::vector<double> out;
    out.reserve(v.size() * n);
    for (std::size_t i = 0; i < n; ++i) {
      out.insert(out.end(), v.begin(), v.end());
    }
    return out;
  }

  std::size_t count_param(const std::string & name, std::int64_t least)
  {
    const std::int64_t value = get_parameter(name).as_int();
    if (value < least) {
      throw std::runtime_error(
        name + " must be a count of at least " + std::to_string(least) + ", got " +
        std::to_string(value));
    }
    return static_cast<std::size_t>(value);
  }

  sentil::Formula resolve_spec()
  {
    const auto formula = get_parameter("spec.formula").as_string();
    const auto name = get_parameter("spec.name").as_string();
    if (!formula.empty()) {
      return sentil::Formula::parse(formula);
    }
    if (!name.empty()) {
      sentil::SpecBuilder builder(name);
      const auto variant = get_parameter("spec.variant").as_string();
      if (!variant.empty()) {
        builder = std::move(builder).with_variant(variant);
      }
      return builder.build_formula();
    }
    throw std::runtime_error("set spec.formula or spec.name");
  }

  void build()
  {
    mode_ = get_parameter("mode").as_string();
    input_width_ = count_param("input_width", 1);
    state_dim_ = count_param("model.state_dim", 0);
    const double dt = get_parameter("model.dt").as_double();
    const auto horizon = count_param("model.horizon", 1);
    const double budget_ms = get_parameter("budget_ms").as_double();
    if (!(std::isfinite(budget_ms) && budget_ms >= 0.0)) {
      throw std::runtime_error(
        "budget_ms must be a finite deadline of zero or more milliseconds, got " +
        std::to_string(budget_ms));
    }
    const auto budget_ns = static_cast<std::uint64_t>(budget_ms * 1e6);
    period_ = std::chrono::duration<double>(1.0 / get_parameter("rate_hz").as_double());

    const auto lower = get_parameter("bounds.lower").as_double_array();
    const auto upper = get_parameter("bounds.upper").as_double_array();
    const bool has_bounds = !lower.empty() && lower.size() == upper.size();

    control_pub_ = create_publisher<msg::Control>(get_parameter("control_topic").as_string(), 10);
    array_pub_ = create_publisher<Float64MultiArray>(
      get_parameter("control_topic").as_string() + "/array", 10);

    if (mode_ == "safety_filter") {
      sentil::Bounds bounds = has_bounds ? sentil::Bounds(lower, upper)
                                         : sentil::Bounds::unbounded(input_width_);
      filter_ = std::make_unique<sentil::SafetyFilter>(std::move(bounds));
      subscribe_state();
      nominal_sub_ = create_subscription<Float64MultiArray>(
        get_parameter("nominal_topic").as_string(), 10,
        [this](Float64MultiArray::SharedPtr msg) { on_nominal(*msg); });
      return;
    }

    const auto vars = get_parameter("model.variables").as_string_array();
    if (state_dim_ == 0 || vars.size() != state_dim_) {
      throw std::runtime_error("model.state_dim must be set and match model.variables");
    }
    auto a = reshape(get_parameter("model.a").as_double_array(), state_dim_, state_dim_, "model.a");
    auto b = reshape(get_parameter("model.b").as_double_array(), state_dim_, input_width_, "model.b");
    const auto x0 = get_parameter("model.x0").as_double_array();
    auto model = sentil::SystemModel::linear(a, b, x0, vars, dt, horizon);
    auto spec = resolve_spec();

    if (mode_ == "receding_horizon") {
      if (has_bounds) {
        sentil::Bounds bounds(lower, upper);
        controller_ = std::make_unique<sentil::Controller>(
          std::move(model), std::move(spec), input_width_, budget_ns, &bounds);
      } else {
        controller_ = std::make_unique<sentil::Controller>(
          std::move(model), std::move(spec), input_width_, budget_ns);
      }
      subscribe_state();
    } else if (mode_ == "open_loop") {
      sentil::SynthesisResult result =
        has_bounds ? synth_with_bounds(model, spec, tile(lower, horizon), tile(upper, horizon))
                   : sentil::synthesis::synthesize(model, spec);
      plan_ = result.input;
      plan_robustness_ = result.robustness;
      plan_holds_ = result.holds;
      RCLCPP_INFO(get_logger(), "synthesized a %zu-input plan, robustness %.4f, holds %s",
                  plan_.size(), plan_robustness_, plan_holds_ ? "true" : "false");
    } else {
      throw std::runtime_error("mode must be receding_horizon, open_loop, or safety_filter");
    }
  }

  sentil::SynthesisResult synth_with_bounds(const sentil::SystemModel & model,
                                            const sentil::Formula & spec,
                                            const std::vector<double> & lower,
                                            const std::vector<double> & upper)
  {
    sentil::Bounds bounds(lower, upper);
    return sentil::synthesis::synthesize(model, spec, &bounds);
  }

  void subscribe_state()
  {
    state_sub_ = create_subscription<Float64MultiArray>(
      get_parameter("state_topic").as_string(), 10,
      [this](Float64MultiArray::SharedPtr msg) { on_state(*msg); });
  }

  void on_state(const Float64MultiArray & msg)
  {
    if (!active_) {
      return;
    }
    std::lock_guard<std::mutex> lock(mutex_);
    state_ = msg.data;
    if (mode_ == "receding_horizon" && controller_) {
      try {
        publish(controller_->control(state_), std::numeric_limits<double>::quiet_NaN(), false, true);
      } catch (const std::exception & e) {
        RCLCPP_WARN_THROTTLE(get_logger(), *get_clock(), 1000, "control failed: %s", e.what());
      }
    }
  }

  void on_nominal(const Float64MultiArray & msg)
  {
    if (!active_ || !filter_) {
      return;
    }
    try {
      publish(filter_->filter(msg.data), std::numeric_limits<double>::quiet_NaN(), false, true);
    } catch (const std::exception & e) {
      RCLCPP_WARN_THROTTLE(get_logger(), *get_clock(), 1000, "filter failed: %s", e.what());
    }
  }

  void emit_open_loop_step()
  {
    if (!active_ || step_ * input_width_ >= plan_.size()) {
      return;
    }
    std::vector<double> u(plan_.begin() + step_ * input_width_,
                          plan_.begin() + (step_ + 1) * input_width_);
    publish(u, plan_robustness_, plan_holds_, true);
    ++step_;
  }

  void publish(const std::vector<double> & input, double robustness, bool holds, bool feasible)
  {
    msg::Control out;
    out.header.stamp = now();
    out.mode = mode_;
    out.input = input;
    out.robustness = robustness;
    out.holds = holds;
    out.feasible = feasible;
    control_pub_->publish(out);

    Float64MultiArray arr;
    arr.data = input;
    array_pub_->publish(arr);
  }

  void teardown()
  {
    timer_.reset();
    state_sub_.reset();
    nominal_sub_.reset();
    control_pub_.reset();
    array_pub_.reset();
    controller_.reset();
    filter_.reset();
    plan_.clear();
    state_.clear();
  }

  std::string mode_;
  std::size_t state_dim_ = 0;
  std::size_t input_width_ = 1;
  std::chrono::duration<double> period_{0.1};
  bool active_ = false;

  std::unique_ptr<sentil::Controller> controller_;
  std::unique_ptr<sentil::SafetyFilter> filter_;
  std::vector<double> plan_;
  double plan_robustness_ = 0.0;
  bool plan_holds_ = false;
  std::size_t step_ = 0;

  std::mutex mutex_;
  std::vector<double> state_;

  rclcpp::Subscription<Float64MultiArray>::SharedPtr state_sub_;
  rclcpp::Subscription<Float64MultiArray>::SharedPtr nominal_sub_;
  rclcpp_lifecycle::LifecyclePublisher<msg::Control>::SharedPtr control_pub_;
  rclcpp_lifecycle::LifecyclePublisher<Float64MultiArray>::SharedPtr array_pub_;
  rclcpp::TimerBase::SharedPtr timer_;
};

}  // namespace sentil_ros

RCLCPP_COMPONENTS_REGISTER_NODE(sentil_ros::ControlNode)