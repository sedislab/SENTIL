#include <chrono>
#include <cstdio>

#include "ara/com/service.h"

namespace {
constexpr ara::com::ServiceId kService{0x6001, 0x0001};
constexpr ara::com::EventId kEvent = 0x8001;
constexpr ara::com::EventGroupId kGroup = 0x0001;
}  // namespace

int main() {
  ara::com::Provider provider("lifecycle_provider", kService);
  provider.offer_event(kEvent, kGroup);
  provider.offer();

  ara::com::Consumer consumer("lifecycle_consumer", kService);
  bool available = false;
  consumer.subscribe(kEvent, kGroup, [](const ara::com::Bytes&) {});
  available = consumer.wait_available(std::chrono::seconds(5));

  std::printf("lifecycle: service became available: %s\n", available ? "yes" : "no");
  return available ? 0 : 1;
}