"""Capture a crowded CARLA drive: signals, odometry, and a forward camera.

Runs under CARLA's Python 3.7 client and imports no ROS.
"""

import argparse
import json
import math
import os
import queue
import random

import numpy as np
from PIL import Image

import carla

CAM_W, CAM_H = 1280, 720


def forward_clearance(ego_tf, ego_loc, others):
    """Distance to the nearest vehicle ahead within a narrow forward corridor."""
    fwd = ego_tf.get_forward_vector()
    right = ego_tf.get_right_vector()
    best = 100.0
    for v in others:
        loc = v.get_location()
        dx, dy = loc.x - ego_loc.x, loc.y - ego_loc.y
        along = dx * fwd.x + dy * fwd.y
        lateral = dx * right.x + dy * right.y
        if along > 0.0 and abs(lateral) < 3.0:
            best = min(best, math.hypot(dx, dy))
    return best


def nearest(ego_loc, actors):
    best = 100.0
    for a in actors:
        best = min(best, ego_loc.distance(a.get_location()))
    return best


def lateral_error(carla_map, tf):
    wp = carla_map.get_waypoint(tf.location, project_to_road=True)
    right = wp.transform.get_right_vector()
    dx = tf.location.x - wp.transform.location.x
    dy = tf.location.y - wp.transform.location.y
    return dx * right.x + dy * right.y


def spawn_traffic(world, tm, bps, points, count):
    out = []
    random.shuffle(points)
    for sp in points[:count]:
        v = world.try_spawn_actor(random.choice(bps), sp)
        if v is not None:
            v.set_autopilot(True, tm.get_port())
            out.append(v)
    return out


def spawn_walkers(world, count):
    bps = world.get_blueprint_library().filter("walker.pedestrian.*")
    ctrl_bp = world.get_blueprint_library().find("controller.ai.walker")
    walkers, controllers = [], []
    for _ in range(count):
        loc = world.get_random_location_from_navigation()
        if loc is None:
            continue
        w = world.try_spawn_actor(random.choice(bps), carla.Transform(loc))
        if w is None:
            continue
        c = world.try_spawn_actor(ctrl_bp, carla.Transform(), attach_to=w)
        walkers.append(w)
        controllers.append(c)
    world.tick()
    for c in controllers:
        c.start()
        c.go_to_location(world.get_random_location_from_navigation())
        c.set_max_speed(1.4 + random.random())
    return walkers, controllers


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="localhost")
    ap.add_argument("--port", type=int, default=2000)
    ap.add_argument("--frames", type=int, default=2400)
    ap.add_argument("--dt", type=float, default=0.05)
    ap.add_argument("--vehicles", type=int, default=40)
    ap.add_argument("--walkers", type=int, default=90)
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--out-dir", default="capture")
    args = ap.parse_args()
    random.seed(args.seed)

    frames_dir = os.path.join(args.out_dir, "frames")
    os.makedirs(frames_dir, exist_ok=True)

    client = carla.Client(args.host, args.port)
    client.set_timeout(120.0)
    world = client.get_world()
    world.set_weather(carla.WeatherParameters.ClearNoon)
    settings = world.get_settings()
    settings.synchronous_mode = True
    settings.fixed_delta_seconds = args.dt
    world.apply_settings(settings)
    tm = client.get_trafficmanager()
    tm.set_synchronous_mode(True)
    tm.set_random_device_seed(args.seed)

    bp_lib = world.get_blueprint_library()
    carla_map = world.get_map()
    points = carla_map.get_spawn_points()

    ego = world.spawn_actor(bp_lib.find("vehicle.tesla.model3"), random.choice(points))
    ego.set_autopilot(True, tm.get_port())
    tm.vehicle_percentage_speed_difference(ego, -40.0)
    tm.ignore_walkers_percentage(ego, 30.0)
    tm.ignore_lights_percentage(ego, 100.0)
    tm.ignore_signs_percentage(ego, 100.0)
    tm.distance_to_leading_vehicle(ego, 1.2)
    tm.auto_lane_change(ego, True)

    cam_bp = bp_lib.find("sensor.camera.rgb")
    cam_bp.set_attribute("image_size_x", str(CAM_W))
    cam_bp.set_attribute("image_size_y", str(CAM_H))
    cam_bp.set_attribute("fov", "90")
    camera = world.spawn_actor(cam_bp, carla.Transform(carla.Location(x=1.4, z=1.5)), attach_to=ego)
    image_queue = queue.Queue()
    camera.listen(image_queue.put)

    collided = {"hit": False}
    coll = world.spawn_actor(bp_lib.find("sensor.other.collision"), carla.Transform(), attach_to=ego)
    coll.listen(lambda _e: collided.__setitem__("hit", True))

    vehicle_bps = [b for b in bp_lib.filter("vehicle.*")
                   if int(b.get_attribute("number_of_wheels")) == 4]
    traffic = spawn_traffic(world, tm, vehicle_bps, list(points), args.vehicles)
    walkers, controllers = spawn_walkers(world, args.walkers)

    records = []
    try:
        for frame in range(args.frames):
            world.tick()
            image = image_queue.get()
            tf = ego.get_transform()
            vel = ego.get_velocity()
            loc = tf.location
            fwd = tf.get_forward_vector()

            arr = np.frombuffer(image.raw_data, dtype=np.uint8).reshape((CAM_H, CAM_W, 4))
            rgb = arr[:, :, :3][:, :, ::-1]
            Image.fromarray(rgb).save(os.path.join(frames_dir, "%06d.jpg" % frame), quality=92)

            others = [v for v in traffic if v.is_alive]
            peds = [w for w in walkers if w.is_alive]
            speed = math.sqrt(vel.x ** 2 + vel.y ** 2 + vel.z ** 2)
            records.append({
                "t": round(frame * args.dt, 4),
                "speed": round(speed, 4),
                "front_distance": round(forward_clearance(tf, loc, others), 3),
                "pedestrian_distance": round(nearest(loc, peds), 3),
                "lateral_error": round(lateral_error(carla_map, tf), 4),
                "x": round(loc.x, 3), "y": round(loc.y, 3),
                "yaw": round(math.radians(tf.rotation.yaw), 5),
                "vx": round(vel.x * fwd.x + vel.y * fwd.y, 4),
                "collision": 1 if collided["hit"] else 0,
            })
            collided["hit"] = False
    finally:
        camera.stop()
        coll.stop()
        for c in controllers:
            if c.is_alive:
                c.stop()
        client.apply_batch([carla.command.DestroyActor(a)
                            for a in [ego, camera, coll] + traffic + walkers + controllers
                            if a.is_alive])
        settings.synchronous_mode = False
        world.apply_settings(settings)

    meta = {
        "town": carla_map.name.split("/")[-1],
        "dt": args.dt,
        "frames": len(records),
        "cam_w": CAM_W, "cam_h": CAM_H,
        "vehicles": len(traffic), "walkers": len(walkers),
        "collisions": sum(r["collision"] for r in records),
        "records": records,
    }
    with open(os.path.join(args.out_dir, "capture.json"), "w") as f:
        json.dump(meta, f)
    print("captured {} frames on {}, {} collisions, {} vehicles, {} walkers".format(
        len(records), meta["town"], meta["collisions"], len(traffic), len(walkers)))


if __name__ == "__main__":
    main()