// BasicMonitor: the simplest SENTIL monitor on a board.
//
// It watches whether a value has stayed positive since power-on, using the
// past-time "historically" operator, which settles to a verdict at every step.
// Feed a reading each loop and read the robustness back: positive is the margin
// by which the property still holds, negative is how far it has been violated.
// Open the Serial Monitor at 115200 baud to follow along.
//
// Past operators suit a real-time loop because they decide from the samples
// already seen. A future operator such as "always" needs samples that have not
// arrived yet, so it would stay provisional until its window closes.
#include <Sentil.h>

static SentilMonitor monitor;
static uint8_t sentil_heap[4096];

// A short script of readings, cycled so the dip below zero is easy to see.
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