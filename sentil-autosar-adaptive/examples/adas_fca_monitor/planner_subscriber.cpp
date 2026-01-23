#include <chrono>
#include <cstdio>
#include <thread>

#include "ara/com/service.h"

#include "sentil_ap/payloads.h"

int main() {
  ara::com::Consumer verdict("planner_subscriber", {0x6001, 0x0001});
  verdict.subscribe(0x8001, 0x0001, [](const ara::com::Bytes& payload) {
    const sentil_ap::Verdict v = sentil_ap::parse_verdict(payload);
    std::printf("verdict t=%.1f robustness=%.3f satisfied=%d\n", v.timestamp, v.robustness_min,
                v.satisfied ? 1 : 0);
  });
  verdict.subscribe(0x8002, 0x0001, [](const ara::com::Bytes& payload) {
    const sentil_ap::Verdict v = sentil_ap::parse_verdict(payload);
    std::printf("VIOLATION at t=%.1f, engaging fallback\n", v.timestamp);
  });

  if (!verdict.wait_available(std::chrono::seconds(5))) {
    std::printf("planner_subscriber: monitor not found\n");
    return 1;
  }
  std::this_thread::sleep_for(std::chrono::seconds(20));
  return 0;
}