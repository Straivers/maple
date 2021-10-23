# Frame Timing

A key goal in the renderer implementation is that each window should be rendered
at some integer multiple of the monitor refresh rate. Having multiple windows
spread across multiple monitors introduces a few problems:

1. Each monitor may run at a different frequency.
2. Monitors running that the same frequency may refresh at different times.
3. Windows may move between monitors, changing their required update frequency.
4. Presentation is a blocking operation.
5. The time a window takes to draw is not known ahead of time (but can be predicted).

## Constraints

1. Each window should update at monitor refresh rate.
2. Each update may cause the window's content to be redrawn.
3. If the window is redrawn, rendering must be complete in the same frame.
4. 

## Implementation

For the moment, implementation of a better frame pacing system is deferred until
GUI rendering is more developed.
