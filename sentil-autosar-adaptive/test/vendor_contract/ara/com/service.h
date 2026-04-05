#ifndef SENTIL_VENDOR_CONTRACT_ARA_COM_SERVICE_H
#define SENTIL_VENDOR_CONTRACT_ARA_COM_SERVICE_H

// The ara::com surface a vendor Adaptive Platform has to supply, declared and never defined.
#include <chrono>
#include <cstdint>
#include <functional>
#include <string>
#include <vector>

namespace ara {
namespace com {

using Bytes = std::vector<std::uint8_t>;
using EventId = std::uint16_t;
using EventGroupId = std::uint16_t;
using MethodId = std::uint16_t;

struct ServiceId {
  std::uint16_t service;
  std::uint16_t instance;
};

class Provider {
 public:
  Provider(const std::string& app_name, ServiceId id);
  ~Provider();

  void offer_event(EventId event, EventGroupId group);
  void on_method(MethodId method, std::function<Bytes(const Bytes&)> handler);
  void offer();
  void notify(EventId event, const Bytes& payload);
};

class Consumer {
 public:
  Consumer(const std::string& app_name, ServiceId id);
  ~Consumer();

  void subscribe(EventId event, EventGroupId group,
                 std::function<void(const Bytes&)> handler);
  bool wait_available(std::chrono::milliseconds timeout);
};

}  // namespace com
}  // namespace ara

#endif  // SENTIL_VENDOR_CONTRACT_ARA_COM_SERVICE_H