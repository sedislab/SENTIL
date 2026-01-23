#ifndef ARA_COM_SERVICE_H
#define ARA_COM_SERVICE_H

#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

#include <vsomeip/vsomeip.hpp>

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

 private:
  std::shared_ptr<vsomeip::application> app_;
  ServiceId id_;
  std::vector<vsomeip::event_t> events_;
  std::map<MethodId, std::function<Bytes(const Bytes&)>> methods_;
};

class Consumer {
 public:
  Consumer(const std::string& app_name, ServiceId id);
  ~Consumer();

  void subscribe(EventId event, EventGroupId group,
                 std::function<void(const Bytes&)> handler);

  bool wait_available(std::chrono::milliseconds timeout);

 private:
  std::shared_ptr<vsomeip::application> app_;
  ServiceId id_;
  std::map<EventId, std::function<void(const Bytes&)>> handlers_;
  std::mutex mutex_;
  std::condition_variable available_cv_;
  bool available_ = false;
};

}  // namespace com
}  // namespace ara

#endif  // ARA_COM_SERVICE_H