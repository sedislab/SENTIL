// BasicMonitor: has the reading stayed positive since power-on?
#include <Sentil.h>

static SentilMonitor monitor;
static uint8_t sentil_heap[4096];

static const double readings[] = {3.0, 1.5, 2.0, -0.5, 4.0};
static const size_t reading_count = sizeof(readings) / sizeof(readings[0]);
static unsigned long step = 0;

void setup() {
  Serial.begin(115200);
  while (!Serial) {
  }
  sentil_embedded_init(sentil_heap, sizeof(sentil_heap));
  sentil_embedded_status_t status = monitor.begin("historically (x > 0)");
  if (status != SENTIL_EMBEDDED_OK) {
    Serial.print("could not build the monitor: ");
    Serial.println(sentil_embedded_status_message(status));
    while (true) {
    }
  }
}

void loop() {
  double x = readings[step % reading_count];
  double packed[1] = {x};
  sentil_embedded_robustness_t robustness;
  if (monitor.update((double)step, packed, 1, robustness) == SENTIL_EMBEDDED_OK) {
    Serial.print("x=");
    Serial.print(x);
    Serial.print("  robustness=");
    Serial.print(robustness.value);
    Serial.println(robustness.satisfied ? "  (holds)" : "  (violated)");
  }
  step++;
  delay(1000);
}