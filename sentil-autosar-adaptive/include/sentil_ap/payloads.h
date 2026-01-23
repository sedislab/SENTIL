/// The Float64Vector the ARXML declares.
struct ControlCommand {
  std::vector<double> command;
};

inline Bytes serialize(const ControlCommand& event) {
  Bytes out;
  detail::put_doubles(out, event.command);
  return out;
}

inline ControlCommand parse_control_command(const Bytes& bytes) {
  detail::Reader reader(bytes);
  ControlCommand event;
  event.command = reader.get_doubles();
  return event;
}

