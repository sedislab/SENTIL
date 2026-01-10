# The running satisfaction probability of a P~p formula, estimated online over the
# lifted particle ensemble, with a Wilson interval at ci_confidence over satisfactions
# out of samples.
std_msgs/Header header
string formula_id
float64 estimate
bool is_concrete
uint64 samples
uint64 satisfactions
float64 ci_lower
float64 ci_upper
float64 ci_confidence