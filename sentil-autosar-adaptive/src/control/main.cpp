#include <atomic>
#include <chrono>
#include <csignal>
#include <thread>

#include "ara/com/service.h"
#include "ara/exec/execution_client.h"
#include "ara/log/log.h"

#include "control_app.hpp"
#include "sentil_ap/payloads.h"

namespace {

constexpr ara::com::ServiceId kControlService{0x6002, 0x0001};
constexpr ara::com::EventId kControlCommandEvent = 0x8003;
constexpr ara::com::EventId kControllerStatusEvent = 0x8004;
constexpr ara::com::MethodId kComputeControlMethod = 0x0001;
constexpr ara::com::EventGroupId kGroup = 0x0001;

std::atomic<bool> g_running{true};
void on_signal(int) { g_running = false; }

}  // namespace

int main() {
  ara::log::Logger log = ara::log::CreateLogger("sentil_control");
  ara::exec::ExecutionClient execution;

  sentil_ap::ControlApp app =
      sentil_ap::ControlApp::shield({0.0, 0.0, -100.0}, {100.0, 100.0, 100.0});

  ara::com::Provider control("sentil_control", kControlService);
  control.offer_event(kControlCommandEvent, kGroup);
  control.offer_event(kControllerStatusEvent, kGroup);
  control.on_method(kComputeControlMethod, [&app, &log, &control](const ara::com::Bytes& request_bytes) {
    try {
      const sentil_ap::ControlRequest request = sentil_ap::parse_control_request(request_bytes);
      const sentil_ap::ControlApp::Outcome outcome = app.compute(request.state, request.nominal);
      if (outcome.status.feasible) {
        control.notify(kControlCommandEvent, sentil_ap::serialize(sentil_ap::ControlCommand{outcome.command}));
      }
      control.notify(kControllerStatusEvent, sentil_ap::serialize(outcome.status));
      return sentil_ap::serialize(sentil_ap::ControlResponse{outcome.command, outcome.status.feasible});
    } catch (const std::exception& error) {
      log.LogError() << "compute control failed: " << error.what();
      return sentil_ap::serialize(sentil_ap::ControlResponse{});
    }
  });
  control.offer();

  std::signal(SIGINT, on_signal);
  std::signal(SIGTERM, on_signal);
  execution.ReportExecutionState(ara::exec::ExecutionState::kRunning);
  log.LogInfo() << "offering the control service";
  while (g_running) {
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
  }
  return 0;
}