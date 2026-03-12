# robustness and holds carry a verdict only in open_loop mode; elsewhere robustness is NaN.
std_msgs/Header header
string mode
float64[] input
float64 robustness
bool holds
bool feasible