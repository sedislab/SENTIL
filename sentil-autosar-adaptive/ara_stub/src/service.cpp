#include "ara/com/service.h"

#include <thread>

namespace ara {
namespace com {
namespace {

std::shared_ptr<vsomeip::payload> make_payload(const Bytes& bytes) {
  auto payload = vsomeip::runtime::get()->create_payload();
  payload->set_data(bytes);
  return payload;
}

Bytes read_payload(const std::shared_ptr<vsomeip::message>& message) {
  const std::shared_ptr<vsomeip::payload> payload = message->get_payload();
  const vsomeip::byte_t* data = payload->get_data();
  return Bytes(data, data + payload->get_length());
}

}  // namespace

Provider::Provider(const std::string& app_name, ServiceId id) : id_(id) {
  app_ = vsomeip::runtime::get()->create_application(app_name);
  app_->init();
  std::thread([app = app_]() { app->start(); }).detach();
}

Provider::~Provider() {
  app_->stop_offer_service(id_.service, id_.instance);
  app_->stop();
}

void Provider::offer_event(EventId event, EventGroupId group) {
  std::set<vsomeip::eventgroup_t> groups{group};
  app_->offer_event(id_.service, id_.instance, event, groups, vsomeip::event_type_e::ET_FIELD);
  events_.push_back(event);
}

void Provider::on_method(MethodId method, std::function<Bytes(const Bytes&)> handler) {
  methods_[method] = std::move(handler);
  app_->register_message_handler(
      id_.service, id_.instance, method,
      [this, method](const std::shared_ptr<vsomeip::message>& request) {
        const Bytes response_bytes = methods_.at(method)(read_payload(request));
        std::shared_ptr<vsomeip::message> response = vsomeip::runtime::get()->create_response(request);
        response->set_payload(make_payload(response_bytes));
        app_->send(response);
      });
}

void Provider::offer() { app_->offer_service(id_.service, id_.instance); }

void Provider::notify(EventId event, const Bytes& payload) {
  app_->notify(id_.service, id_.instance, event, make_payload(payload));
}

Consumer::Consumer(const std::string& app_name, ServiceId id) : id_(id) {
  app_ = vsomeip::runtime::get()->create_application(app_name);
  app_->init();
  app_->register_availability_handler(
      id_.service, id_.instance,
      [this](vsomeip::service_t, vsomeip::instance_t, bool available) {
        std::lock_guard<std::mutex> lock(mutex_);
        available_ = available;
        available_cv_.notify_all();
      });
  app_->request_service(id_.service, id_.instance);
  std::thread([app = app_]() { app->start(); }).detach();
}

Consumer::~Consumer() {
  app_->release_service(id_.service, id_.instance);
  app_->stop();
}

void Consumer::subscribe(EventId event, EventGroupId group,
                         std::function<void(const Bytes&)> handler) {
  handlers_[event] = std::move(handler);
  std::set<vsomeip::eventgroup_t> groups{group};
  app_->request_event(id_.service, id_.instance, event, groups, vsomeip::event_type_e::ET_FIELD);
  app_->register_message_handler(
      id_.service, id_.instance, event,
      [this, event](const std::shared_ptr<vsomeip::message>& message) {
        handlers_.at(event)(read_payload(message));
      });
  app_->subscribe(id_.service, id_.instance, group);
}

bool Consumer::wait_available(std::chrono::milliseconds timeout) {
  std::unique_lock<std::mutex> lock(mutex_);
  return available_cv_.wait_for(lock, timeout, [this]() { return available_; });
}

}  // namespace com
}  // namespace ara