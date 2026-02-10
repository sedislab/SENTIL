# Experiments

The case studies and deployments behind the numbers in [../docs/CLAIMS.md](../docs/CLAIMS.md). Each directory has its own README with the model, the specifications, and the command that regenerates its artifact into that directory's `results/`. The claims ledger records the expected value and tolerance for each, and [../docs/REPRODUCE.md](../docs/REPRODUCE.md) explains the tiers and how to run them.

## The four case studies

| Directory | What it shows | Tier |
| --- | --- | --- |
| [carla_driving](carla_driving) | monitoring an autonomous drive in CARLA against a compound deterministic and probabilistic spec | CPU replay (recording is GPU) |
| [glucose_control](glucose_control) | a closed-loop insulin controller checked against clinical safety specs, deterministically and under CGM noise | CPU |
| [circadian_gene_network](circadian_gene_network) | verifying sustained oscillation in a Barkai-Leibler circadian gene network | CPU |
| [embedded_deployment](embedded_deployment) | per-cycle latency of the monitor as a safety supervisor on a Raspberry Pi 4 | hardware-bound (Pi 4) |

## Platform integrations

| Directory | Platform |
| --- | --- |
| [apollo_aeb_monitoring](apollo_aeb_monitoring) | Baidu Apollo, emergency braking |
| [apollo_cyber_monitoring](apollo_cyber_monitoring) | Apollo Cyber RT, live component monitoring |
| [autosar_fca_monitoring](autosar_fca_monitoring) | AUTOSAR Adaptive Platform, forward-collision avoidance |

## Synthesis and robotics

| Directory | What it shows |
| --- | --- |
| [robot_arm_synthesis](robot_arm_synthesis) | synthesizing a robot-arm controller from a spec, in ROS |
| [robot_nav_monitoring](robot_nav_monitoring) | monitoring mobile-robot navigation, in ROS |
| [uav_geofence](uav_geofence) | UAV geofence and collision avoidance |

## Running

The CPU studies run directly and finish in seconds to minutes, for instance:

```
python experiments/glucose_control/glucose_control.py
python experiments/circadian_gene_network/circadian_gene_network.py
```

The Raspberry Pi latency is bound to that board, and the ROS, Apollo, and AUTOSAR studies need their respective platforms; each directory's README states what it needs and what to expect where the hardware is not available.