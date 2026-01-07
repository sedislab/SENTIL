# Case study: autonomous driving in CARLA

An autonomous vehicle has to keep its lane, keep clear of traffic, and not hit anyone, all from a moving platform in a world full of other agents whose next move it cannot know for certain. This case study runs SENTIL as the runtime monitor on a vehicle driving through the CARLA simulator, and shows two things: the monitor is fast enough to run inside the control loop, and its probabilistic layer catches a risk that a deterministic monitor reports as safe.

## Two parts

Recording needs CARLA and a GPU; monitoring needs neither. The two are separate on purpose, so the verdicts reproduce from the committed trace.

`record_drive.py` drives an ego vehicle under the CARLA Traffic Manager through an urban map shared with traffic and pedestrians, and records the monitored signals each frame: lateral lane error, distance to the nearest vehicle and pedestrian, speed, the ego pose, and the nearest pedestrian's relative motion. It runs under Python 3.7 with the carla 0.9.15 client. Because it talks to the server only over the network, it works the same whether CARLA runs natively, in Docker, or in an Apptainer image, locally or remote.

`monitor_drive.py` reads that trace and checks it with SENTIL. It runs on plain CPU.

## The specification

```
always (|lateral_error| < 0.3)          lane keeping, within 0.3 m
always (obstacle_distance > 5.0)         clearance, 5 m from the nearest agent
always (speed < 50)                      urban speed limit, km/h
P>=0.99(always[0,10] (no collision))     collision-free over the next 10 s under
                                         uncertainty about where pedestrians go
```

The fourth conjunct is the point. At each frame it predicts the nearest pedestrian's path over a ten-second lookahead, lifts that prediction by its uncertainty (a pedestrian can change direction, and the further ahead you look the less sure you are), and estimates the probability that the vehicle stays clear. A deterministic monitor sees only the current distance, which can sit comfortably above 5 m while a crossing pedestrian makes a collision likely within seconds.

## Results

The committed trace is a 300-second drive (6000 frames at 20 Hz) through the Town10HD urban map, under the CARLA Traffic Manager, sharing the road with 40 vehicles and 30 pedestrians. The verdicts are in `results/verdicts.json` and the plot in `results/drive.png`, both regenerated from `results/drive.json`.

Latency is what makes online monitoring viable. The three deterministic conjuncts run together on the streaming monitor at about 0.54 microseconds per sample, a sustained 1.8 million samples per second, far beyond the few hundred hertz a control loop publishes at. The probabilistic conjunct, the expensive one, runs at a median of about 0.65 ms per frame and a 99th percentile of about 0.89 ms, inside the 2 ms closed-loop deadline. For comparison, the STORM paper reports an RTAMT-based monitor at about 47 ms per frame on the same workload, which misses the deadline by more than twenty times. These figures are measured on the machine that runs the monitor, not the GPU node that runs CARLA, since the monitor is CPU work.

The deterministic monitor flags what the Traffic Manager actually does. Its lane keeping is not tuned to 0.3 m, so lateral error spikes past the bound at a couple of turns, and it lets the car approach other traffic closer than 5 m, so both conjuncts report violations. That is the autopilot's real behavior, and the monitor reports it as it is.

The probabilistic check is the one that earns its place, at the pedestrian encounter near t = 146 s. There the deterministic clearance still holds, 5.7 m to the nearest agent, so a classical monitor reports no problem. SENTIL puts the collision-free probability over the next ten seconds at essentially zero, because the pedestrian's predicted path, under its uncertainty, runs into the car's own recorded path inside the lookahead. The car did pass within about 1.9 m of a pedestrian on this drive. The probabilistic verdict sat near 1.0 the whole time except at the few real encounters, where it dropped sharply, which is exactly the behavior you want from a risk monitor. That is a hazard a deterministic check cannot express, and it is the strongest argument for probabilistic monitoring in this domain.

## Run it

Monitoring, from the committed trace, no GPU:

```
python experiments/carla_driving/monitor_drive.py --trace experiments/carla_driving/results/drive.json
```

Recording a fresh trace, against any CARLA 0.9.15 server running on `localhost:2000`, using whichever map the server has loaded:

```
python record_drive.py --frames 6000 --out results/drive.json
```

The recorder needs the `carla` client package, which CARLA ships for Python 3.7, so run it under a Python 3.7 interpreter. The monitor needs `sentil`, NumPy, and Matplotlib, and runs under any recent Python.