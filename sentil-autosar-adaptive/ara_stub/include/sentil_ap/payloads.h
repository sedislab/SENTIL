#ifndef SENTIL_AP_PAYLOADS_H
#define SENTIL_AP_PAYLOADS_H

// The SENTIL service payloads and their byte encoding, shared by the apps, the examples,
// and the tests. The encoding is a plain little-endian packing used as the SOME/IP event
// and method payload, so any ara::com transport carries the same bytes.
#include <cstdint>
#include <cstring>
#include <stdexcept>
#include <string>
#include <vector>

namespace sentil_ap {

/// A frame of named scalar readings on one timestamp, the monitor's input.
struct SignalFrame {
  double t = 0.0;
  std::vector<std::string> names;
  std::vector<double> values;
};

/// The monitor's output for one formula, matching sentil::Robustness plus the running
/// probability and its interval.
struct Verdict {
  double timestamp = 0.0;
  double robustness_min = 0.0;
  double robustness_max = 0.0;
  bool satisfied = false;
  bool is_concrete = false;
  double probability = 0.0;
  double ci_lower = 0.0;
  double ci_upper = 0.0;
};

/// The control app's status alongside each command.
struct ControllerStatus {
  bool deadline_met = false;
  bool feasible = false;
  bool cbf_active = false;
  double robustness = 0.0;
};

using Bytes = std::vector<std::uint8_t>;

namespace detail {

inline void put_double(Bytes& out, double value) {
  const auto* p = reinterpret_cast<const std::uint8_t*>(&value);
  out.insert(out.end(), p, p + sizeof(double));
}

inline void put_u32(Bytes& out, std::uint32_t value) {
  for (int i = 0; i < 4; ++i) {
    out.push_back(static_cast<std::uint8_t>((value >> (8 * i)) & 0xFF));
  }
}

inline void put_string(Bytes& out, const std::string& value) {
  put_u32(out, static_cast<std::uint32_t>(value.size()));
  out.insert(out.end(), value.begin(), value.end());
}

inline void put_doubles(Bytes& out, const std::vector<double>& values) {
  put_u32(out, static_cast<std::uint32_t>(values.size()));
  for (double value : values) {
    put_double(out, value);
  }
}

/// A bounds-checked cursor over a byte buffer; every read throws on an overrun rather
/// than reading past the end, so a truncated payload never corrupts memory.
class Reader {
 public:
  explicit Reader(const Bytes& bytes) : bytes_(bytes) {}

  double get_double() {
    need(sizeof(double));
    double value = 0.0;
    std::memcpy(&value, bytes_.data() + pos_, sizeof(double));
    pos_ += sizeof(double);
    return value;
  }

  std::uint32_t get_u32() {
    need(4);
    std::uint32_t value = 0;
    for (int i = 0; i < 4; ++i) {
      value |= static_cast<std::uint32_t>(bytes_[pos_ + i]) << (8 * i);
    }
    pos_ += 4;
    return value;
  }

  std::string get_string() {
    const std::uint32_t len = get_u32();
    need(len);
    std::string value(bytes_.begin() + pos_, bytes_.begin() + pos_ + len);
    pos_ += len;
    return value;
  }

  bool get_bool() {
    need(1);
    return bytes_[pos_++] != 0;
  }

  std::vector<double> get_doubles() {
    const std::uint32_t count = get_u32();
    std::vector<double> out;
    out.reserve(count);
    for (std::uint32_t i = 0; i < count; ++i) {
      out.push_back(get_double());
    }
    return out;
  }

 private:
  void need(std::size_t count) const {
    if (pos_ + count > bytes_.size()) {
      throw std::runtime_error("truncated SENTIL payload");
    }
  }

  const Bytes& bytes_;
  std::size_t pos_ = 0;
};

}  // namespace detail

inline Bytes serialize(const SignalFrame& frame) {
  Bytes out;
  detail::put_double(out, frame.t);
  detail::put_u32(out, static_cast<std::uint32_t>(frame.names.size()));
  for (const std::string& name : frame.names) {
    detail::put_string(out, name);
  }
  detail::put_u32(out, static_cast<std::uint32_t>(frame.values.size()));
  for (double value : frame.values) {
    detail::put_double(out, value);
  }
  return out;
}

inline SignalFrame parse_signal_frame(const Bytes& bytes) {
  detail::Reader reader(bytes);
  SignalFrame frame;
  frame.t = reader.get_double();
  const std::uint32_t name_count = reader.get_u32();
  frame.names.reserve(name_count);
  for (std::uint32_t i = 0; i < name_count; ++i) {
    frame.names.push_back(reader.get_string());
  }
  const std::uint32_t value_count = reader.get_u32();
  frame.values.reserve(value_count);
  for (std::uint32_t i = 0; i < value_count; ++i) {
    frame.values.push_back(reader.get_double());
  }
  return frame;
}

inline Bytes serialize(const Verdict& verdict) {
  Bytes out;
  detail::put_double(out, verdict.timestamp);
  detail::put_double(out, verdict.robustness_min);
  detail::put_double(out, verdict.robustness_max);
  out.push_back(verdict.satisfied ? 1 : 0);
  out.push_back(verdict.is_concrete ? 1 : 0);
  detail::put_double(out, verdict.probability);
  detail::put_double(out, verdict.ci_lower);
  detail::put_double(out, verdict.ci_upper);
  return out;
}

inline Verdict parse_verdict(const Bytes& bytes) {
  detail::Reader reader(bytes);
  Verdict verdict;
  verdict.timestamp = reader.get_double();
  verdict.robustness_min = reader.get_double();
  verdict.robustness_max = reader.get_double();
  verdict.satisfied = reader.get_bool();
  verdict.is_concrete = reader.get_bool();
  verdict.probability = reader.get_double();
  verdict.ci_lower = reader.get_double();
  verdict.ci_upper = reader.get_double();
  return verdict;
}

/// A ComputeControl request: the current state and the nominal command to shield.
struct ControlRequest {
  std::vector<double> state;
  std::vector<double> nominal;
};

/// A ComputeControl response: the command and whether a feasible one was found.
struct ControlResponse {
  std::vector<double> command;
  bool feasible = false;
};

inline Bytes serialize(const ControlRequest& request) {
  Bytes out;
  detail::put_doubles(out, request.state);
  detail::put_doubles(out, request.nominal);
  return out;
}

inline ControlRequest parse_control_request(const Bytes& bytes) {
  detail::Reader reader(bytes);
  ControlRequest request;
  request.state = reader.get_doubles();
  request.nominal = reader.get_doubles();
  return request;
}

inline Bytes serialize(const ControlResponse& response) {
  Bytes out;
  detail::put_doubles(out, response.command);
  out.push_back(response.feasible ? 1 : 0);
  return out;
}

inline ControlResponse parse_control_response(const Bytes& bytes) {
  detail::Reader reader(bytes);
  ControlResponse response;
  response.command = reader.get_doubles();
  response.feasible = reader.get_bool();
  return response;
}

}  // namespace sentil_ap

#endif  // SENTIL_AP_PAYLOADS_H