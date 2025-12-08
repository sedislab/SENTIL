// StreamingThreshold: a windowed safety alarm on an analog input.
//
// It checks that the reading on A0 has stayed below a limit over the last few
// samples, using a bounded past-time window, and turns on the built-in LED when
// that fails. The window means one noisy spike has to persist to trip the alarm,
// so a single stray sample does not cause a false alert. Wire a sensor or a
// potentiometer to A0.
#include <Sentil.h>

static SentilMonitor monitor;
static uint8_t sentil_heap[4096];
static unsigned long step = 0;

void setup() {
  pinMode(LED_BUILTIN, OUTPUT);
  sentil_embedded_init(sentil_heap, sizeof(sentil_heap));
  // analogRead returns 0..1023; alarm if the recent window approached the rail.
  monitor.begin("historically[0, 8](level < 900)");
}

void loop() {
  double level = (double)analogRead(A0);
  double packed[1] = {level};
  sentil_embedded_robustness_t robustness;
  if (monitor.update((double)step, packed, 1, robustness) == SENTIL_EMBEDDED_OK) {
    digitalWrite(LED_BUILTIN, robustness.satisfied ? LOW : HIGH);
  }
  step++;
  delay(50);
}