#include <chrono>
#include <cstdio>
#include <thread>

#include "ara/com/service.h"

#include "sentil_ap/payloads.h"

int main() {
  ara::com::Provider signals("perception_publisher", {0x6000, 0x0001});
  signals.offer_event(0x8000, 0x0001);
  signals.offer();

  double gap = 30.0;
  for (double t = 0.0; t < 60.0; t += 0.1) {
    sentil_ap::SignalFrame frame;
    frame.t = t;
    frame.names = {"front_gap"};
    frame.values = {gap};
    signals.notify(0x8000, sentil_ap::serialize(frame));
    gap = gap > 1.0 ? gap - 0.5 : gap;
    std::this_thread::sleep_for(std::chrono::milliseconds(20));
  }
  std::printf("perception_publisher: done\n");
  return 0;
}