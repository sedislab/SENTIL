#include <atomic>
#include <chrono>
#include <csignal>
#include <thread>

#include "ara/com/service.h"
#include "ara/exec/execution_client.h"
#include "ara/log/log.h"

#include "monitor_app.hpp"
#include "sentil_ap/payloads.h"

namespace {

constexpr ara::com::ServiceId kSignalService{0x6000, 0x0001};
constexpr ara::com::ServiceId kVerdictService{0x6001, 0x0001};
constexpr ara::com::EventId kSignalFrameEvent = 0x8000;
constexpr ara::com::EventId kVerdictEvent = 0x8001;
constexpr ara::com::EventId kViolationEvent = 0x8002;
constexpr ara::com::MethodId kSetSpecificationMethod = 0x0001;
constexpr ara::com::EventGroupId kGroup = 0x0001;

std::atomic<bool> g_running{true};
void on_signal(int) { g_running = false; }

}  // namespace

int main() {
  ara::log::Logger log = ara::log::CreateLogger("sentil_monitor");
  ara::exec::ExecutionClient execution;

  sentil_ap::MonitorApp app;
  app.add("speed_limit", "always[0, 10] (speed < 30.0)");

  ara::com::Provider verdict("sentil_monitor", kVerdictService);
  verdict.offer_event(kVerdictEvent, kGroup);
  verdict.offer_event(kViolationEvent, kGroup);
  verdict.on_method(kSetSpecificationMethod, [&log](const ara::com::Bytes&) {
    log.LogInfo() << "SetSpecification received";
    return ara::com::Bytes{1};  // accepted
  });
  verdict.offer();

  bool last_satisfied = true;
  ara::com::Consumer signals("sentil_monitor", kSignalService);
  signals.subscribe(kSignalFrameEvent, kGroup, [&](const ara::com::Bytes& payload) {
    try {
      const sentil_ap::SignalFrame frame = sentil_ap::parse_signal_frame(payload);
      for (const auto& entry : app.on_frame(frame)) {
        const sentil_ap::Verdict& result = entry.second;
        verdict.notify(kVerdictEvent, sentil_ap::serialize(result));
        if (last_satisfied && !result.satisfied) {
          verdict.notify(kViolationEvent, sentil_ap::serialize(result));
        }
        last_satisfied = result.satisfied;
      }
    } catch (const std::exception& error) {
      log.LogError() << "frame failed: " << error.what();
    }
  });

  std::signal(SIGINT, on_signal);
  std::signal(SIGTERM, on_signal);
  execution.ReportExecutionState(ara::exec::ExecutionState::kRunning);
  log.LogInfo() << "offering the verdict service";
  while (g_running) {
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
  }
  return 0;
}