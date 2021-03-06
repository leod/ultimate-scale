# TODO
## Editor
- Show some info on mouse over of blocks
- Fast scrolling when zoomed out
- Reconsider combining pipes when dragging/dropping/placing
- No red outline when placing same object
- Smooth camera movement
- Don't allow deleting in multiple layers without moving the mouse
- Don't scroll on C-s or C-a

## Rendering
- Highlight wind-wind interactions
- Improve pillars
- Handle shadow mapping on large maps
- Line rendering does not correctly account for FOV!
- Have pipes show the difference between wind in/out holes somehow

## Gameplay
- Campaign mode

## Optimization
- Figure out a way to use pareen without boxes
- Render outlines as boxes instead of 12 lines
- See if we need triple-buffering for update/draw threading
- Some cheap culling heuristic
- Don't use `from_euler_angles` for axis-aligned stuff
- Particle rendering without geometry shaders
- Wind LoD
- Better streaming of instance and particle data
    - Persistent mapping + triple buffering?
- Precompute inverse transform for normals

## Execution
- Bug when flinging a blip up
- Disallow blip moving through blip

### Block ideas
- Fixed-size blip buffer
- Stateful left/right pipe 
- Explicit block for falling blips
- Perhaps make funnel properties part of the pipe

### UI
- Replace imgui

## Sound
 - _Anything at all_!

## Level Ideas
### Introductory
- Only let through green
- Flip color
- Even/odd
- Append green

### Intermediate
- Splitting input into two outputs
- Splitting input into head/tail
- Something about buffering?

### Hard

## Other
- Revise `TickTime`, manipulating it is needlessly tedious
