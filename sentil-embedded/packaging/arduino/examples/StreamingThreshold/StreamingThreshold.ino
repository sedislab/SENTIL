// StreamingThreshold: a windowed safety alarm on an analog input.
#include <Sentil.h>

static SentilMonitor monitor;
static uint8_t sentil_heap[4096];
static unsigned long step = 0;

void setup() {
  pinMode(LED_BUILTIN, OUTPUT);
  sentil_embedded_init(sentil_heap, sizeof(sentil_heap));
  // analogRead returns 0..1023.
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