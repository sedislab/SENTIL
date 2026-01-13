# sentil-apollo

Runtime verification and controller synthesis for Baidu Apollo, built on the SENTIL engine and packaged as a Cyber RT module.

It ships two components. The monitor watches Apollo channels against Signal Temporal Logic and probabilistic STL specifications and publishes a verdict per formula. The control component synthesizes a command from a specification or shields Apollo's nominal command so it stays within the specification's bounds. Both read their formulas and channel-to-variable mapping from a protobuf-text config, and both carry the compiled SENTIL core inside the module, so the build needs no Rust toolchain.

This directory installs into an Apollo workspace as `modules/sentil`.