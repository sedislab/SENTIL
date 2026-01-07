"""Record an autonomous-driving trace from a live CARLA server.

Runs under Python 3.7 with the carla 0.9.15 client.
"""

import argparse
import json
import math
import random

import carla


def nearest(ego_loc, actors):
    """Smallest distance from the ego to any actor in the list."""
    best = math.inf
    nearest_actor = None
    for a in actors:
        d = ego_loc.distance(a.get_location())
        if d < best:
            best, nearest_actor = d, a
    return best, nearest_actor


def lateral_error(carla_map, transform):
    """Signed perpendicular distance from the lane center, in metres."""
    wp = carla_map.get_waypoint(transform.location, project_to_road=True)
    center = wp.transform.location
    right = wp.transform.get_right_vector()
    dx = transform.location.x - center.x
    dy = transform.location.y - center.y
    return dx * right.x + dy * right.y


def spawn_traffic(world, tm, blueprints, points, count):
    vehicles = []
    for sp in random.sample(points, min(count, len(points))):
        bp = random.choice(blueprints)
        v = world.try_spawn_actor(bp, sp)
        if v is not None:
            v.set_autopilot(True, tm.get_port())
            vehicles.append(v)
    return vehicles


def spawn_walkers(world, count):
    """Spawn pedestrians with AI controllers that walk to random navigation points."""
    bps = world.get_blueprint_library().filter("walker.pedestrian.*")
    controller_bp = world.get_blueprint_library().find("controller.ai.walker")
    walkers, controllers = [], []
    for _ in range(count):
        loc = world.get_random_location_from_navigation()
        if loc is None:
            continue
        walker = world.try_spawn_actor(random.choice(bps), carla.Transform(loc))
        if walker is None:
            continue
        ctrl = world.try_spawn_actor(controller_bp, carla.Transform(), attach_to=walker)
        walkers.append(walker)
        controllers.append(ctrl)
    world.tick()
    for ctrl in controllers:
        ctrl.start()
        ctrl.go_to_location(world.get_random_location_from_navigation())
        ctrl.set_max_speed(1.4)
    return walkers, controllers


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="localhost")
    ap.add_argument("--port", type=int, default=2000)
    ap.add_argument("--frames", type=int, default=6000)
    ap.add_argument("--dt", type=float, default=0.05)
    ap.add_argument("--vehicles", type=int, default=40)
    ap.add_argument("--walkers", type=int, default=30)
    ap.add_argument("--seed", type=int, default=2024)
    ap.add_argument("--out", default="results/drive.json")
    args = ap.parse_args()
    random.seed(args.seed)

    client = carla.Client(args.host, args.port)
    client.set_timeout(120.0)
    world = client.get_world()

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

    ego_bp = bp_lib.find("vehicle.tesla.model3")
    ego = world.spawn_actor(ego_bp, random.choice(points))
    ego.set_autopilot(True, tm.get_port())

    collided = {"hit": False}
    coll_bp = bp_lib.find("sensor.other.collision")
    coll = world.spawn_actor(coll_bp, carla.Transform(), attach_to=ego)
    coll.listen(lambda _e: collided.__setitem__("hit", True))

    vehicle_bps = [b for b in bp_lib.filter("vehicle.*") if int(b.get_attribute("number_of_wheels")) == 4]
    traffic = spawn_traffic(world, tm, vehicle_bps, points, args.vehicles)
    walkers, controllers = spawn_walkers(world, args.walkers)

    times, lat, veh_dist, ped_dist, speed = [], [], [], [], []
    ego_xy, ped_rel, collisions = [], [], []
    try:
        for frame in range(args.frames):
            world.tick()
            tf = ego.get_transform()
            vel = ego.get_velocity()
            loc = tf.location

            others = [v for v in traffic if v.is_alive]
            peds = [w for w in walkers if w.is_alive]
            vd, _ = nearest(loc, others)
            pd, near_ped = nearest(loc, peds)

            times.append(round(frame * args.dt, 4))
            lat.append(round(lateral_error(carla_map, tf), 4))
            veh_dist.append(round(vd, 3))
            ped_dist.append(round(pd, 3))
            speed.append(round(3.6 * math.sqrt(vel.x ** 2 + vel.y ** 2 + vel.z ** 2), 3))
            ego_xy.append([round(loc.x, 2), round(loc.y, 2)])
            if near_ped is not None and math.isfinite(pd):
                pv = near_ped.get_velocity()
                pl = near_ped.get_location()
                ped_rel.append([round(pl.x - loc.x, 2), round(pl.y - loc.y, 2),
                                round(pv.x, 3), round(pv.y, 3)])
            else:
                ped_rel.append([None, None, 0.0, 0.0])
            collisions.append(1 if collided["hit"] else 0)
            collided["hit"] = False
    finally:
        coll.stop()
        for c in controllers:
            if c.is_alive:
                c.stop()
        client.apply_batch([carla.command.DestroyActor(a)
                            for a in [ego, coll] + traffic + walkers + controllers if a.is_alive])
        settings.synchronous_mode = False
        world.apply_settings(settings)

    trace = {
        "source": "carla",
        "town": carla_map.name.split("/")[-1],
        "dt": args.dt,
        "frames": len(times),
        "collision_total": int(sum(collisions)),
        "signals": {
            "t": times,
            "lateral_error": lat,
            "vehicle_distance": veh_dist,
            "pedestrian_distance": ped_dist,
            "speed": speed,
            "ego_xy": ego_xy,
            "pedestrian_relative": ped_rel,
            "collision": collisions,
        },
    }
    with open(args.out, "w") as f:
        json.dump(trace, f)
    print("wrote {} frames to {} ({} collisions)".format(len(times), args.out, sum(collisions)))


if __name__ == "__main__":
    main()