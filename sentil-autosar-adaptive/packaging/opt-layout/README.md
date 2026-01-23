# On-ECU layout

The `.deb` and `.rpm` install the package under `/opt/sentil`:

```
/opt/sentil/
├── bin/{sentil_monitor, sentil_control}
├── lib/libsentil.so
├── manifest/{machine, sentil_monitor.exec, sentil_monitor.si, sentil_control.exec, sentil_control.si}.json
└── etc/vsomeip/{sentil_monitor.json, sentil_control.json}
```

Each process is pointed at its config through `SENTIL_AP_MANIFEST`, or `VSOMEIP_CONFIGURATION` in stub mode. Distribution is the `.deb` and `.rpm` for apt, yum, and pacman, a zipped release for a direct drop, and the ARXML plus manifest set as the AUTOSAR deliverable an integrator imports into their own AP project.