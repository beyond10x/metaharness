# Declared context on the Claude adapter

The existing RunSpec context list is also accepted by the Claude adapter. Each named UTF-8 file
is read before launch, relative to the declared working directory, and appended as explicitly
delimited input. Missing or oversized files refuse the launch. The combined input is bounded to
two megabytes. Context is data and carries no extra tool authority.

A prompt larger than 64 KiB travels through a finite scratch file connected to the child's stdin,
with print mode enabled and no positional prompt. Smaller prompts retain the existing argv shape.
The plan records the exact initial input; the runner opens that finite file so there is no open
pipe to wait on and no pipe-buffer deadlock. Tool decisions continue over the unchanged hook seam.
This changes no event, frame or steering wire shape. The new stdin route requires a live vendor
smoke before being called verified against any vendor version.


Validation: the offline workspace gate passes. A live neutral fixture produced a cited decision
through a downstream kernel verifier, and a larger declared context was delivered over stdin.
This does not establish complete adapter conformance against every vendor version.
