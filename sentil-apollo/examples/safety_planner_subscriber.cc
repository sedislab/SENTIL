#include <memory>

#include "cyber/cyber.h"

#include "modules/sentil/proto/sentil_status.pb.h"

namespace apollo {
namespace sentil {

class SafetyPlannerSubscriber : public apollo::cyber::Component<> {
 public:
  bool Init() override {
    reader_ = node_->CreateReader<SentilStatus>(
        "/apollo/sentil/status",
        [this](const std::shared_ptr<SentilStatus>& status) { OnStatus(*status); });
    return reader_ != nullptr;
  }

 private:
  void OnStatus(const SentilStatus& status) {
    if (!status.all_satisfied()) {
      EngageFallback(status);
      return;
    }
    for (const FormulaResult& result : status.results()) {
      if (result.has_prob_result() && result.prob_result().interval().lower() < kProbabilityFloor) {
        AWARN << "sentil: formula " << result.id() << " probability interval below threshold";
        EngageFallback(status);
        return;
      }
    }
  }

  void EngageFallback(const SentilStatus& status) {
    AWARN << "sentil: engaging fallback";
    for (const FormulaResult& result : status.results()) {
      if (result.satisfied()) {
        continue;
      }
      if (result.robustness().is_concrete()) {
        AERROR << "  violated: " << result.expression();
      } else {
        AWARN << "  undecided: " << result.expression();
      }
    }
  }

  static constexpr double kProbabilityFloor = 0.95;
  std::shared_ptr<apollo::cyber::Reader<SentilStatus>> reader_;
};

CYBER_REGISTER_COMPONENT(SafetyPlannerSubscriber)

}  // namespace sentil
}  // namespace apollo