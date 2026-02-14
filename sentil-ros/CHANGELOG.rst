^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
Changelog for package sentil_ros
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

0.3.0
-----
* First release. A lifecycle monitor node that watches ROS 2 topic streams against
  STL and PrSTL specifications, with generic introspection-based subscriptions, a
  dotted-path field extractor, QoS matching to each publisher, per-formula verdict
  topics, and diagnostics. Includes launch files, example configurations, and a
  container-agnostic CARLA example.
* A lifecycle control node that synthesizes control from a spec and actuates it on
  ROS 2 topics, with five modes: receding_horizon online control under a deadline,
  open_loop trajectory synthesis, a safety_filter control-barrier shield over a
  nominal command, witness counterexample search, and chance-constraint validation.
  Ships the Control message, the sentil_control launch file, and the control_loop
  example.