# A synthesized control command. robustness and holds carry a real verdict only in
# open_loop mode, where the whole input sequence is synthesized against the spec; the
# online modes (receding_horizon, safety_filter) return an input without a verdict, so
# they report robustness as NaN and holds as false. feasible marks that an input was
# produced.
std_msgs/Header header
string mode
float64[] input
float64 robustness
bool holds
bool feasible