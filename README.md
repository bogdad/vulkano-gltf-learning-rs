# vulkano-gltf-learning-rs
[https://github.com/bogdad/vulkano-gltf-learning-rs](https://github.com/bogdad/vulkano-gltf-learning-rs)

## Questions:

- [ ] how do we make terrain under the clouds
- [ ] how do we make clouds transparent
- [?] how do we add power lines to each sky segment
- [x] how do we add power line
- [x] how do we make sound environment
- [x] how do we make clouds seamless
- [x] how do we debug skybox
- [x] how do we mouse look
- [ ] how do we create sunrise
- [x] how do we create skybox 2
- [x] how do we create skybox
- [x] how do we plop the light-sources
- [x] how do we write text on things in 3d
- [x] how do we generate random "cloud like landcapes" on the fly
- [ ] how do vertex normals work
- [x] how do coordinate systems work

## how do we add power lines to each sky segment

in progress:
![power lines in segments, very weird geometry](./images/19.png)

## how do we add power line

![power line, in full](./images/18.png)

story:

we can get a free model of a power line here https://3dsky.org/3dmodels/show/liep_2

and we get this

![power line isolator](./images/17.png)

because our gltf importer only gets one primitive of many. 

after some feedling, and the bug was - when a mesh consists of primitives, the indices of vertices in each primitive starts from zero, i.e. there is no global vertex list, was very puzzled until saw [this](https://github.com/adrien-ben/gltf-viewer-rs/blob/ee6454b2ce1c666037ee7e8704bb46e00f5b94cc/model/src/mesh.rs#L157)


## how do we make sound environment

we are trying to play a sound of the wind
https://freesound.org/people/Huggy13ear/sounds/138970/ by Huggy13ear.

## how do we make clouds seamless 2

The proper thing to do would be when generating next cloud rectangle - pass in references to neighbours so we could make sure neighbour left border for example is equal to our right border.

![squares equal at the border](./images/16.png)

## how do we make clouds seamless

As a first approximation we make Z for clouds equal to 0.0 on the borders of the squares. does not look very good.

![squares zero at the border](./images/15.png)

## how do we debug sky box

and the last problem was 512 size of skybox texture vs. real 1024.

![skybox](./images/14.png)

before that the problem was that we did not wait for the skybox texture future, and as soon as we wait - we get skybox!

![skybox](./images/13.png)

still looks buggy, but cool.

## how do we mouse look

to debug skybox seems its very cool to have mouse look,
now we have one!
![mouse look](./images/12.png)

## how do we create skybox 2

we have some skybox!
![skybox](./images/11.png)

probably a bug as it needs to look like this:
![skybox](./assets/interstellar_skybox/xneg.png)
(by Jockum Skoglund aka hipshot)

## how do we create skybox

made an attempt at skyboxing.
The general idea is:
have 2 render subpasses, 
one normal, as before, with vertex and fragrment shader creating geometry and then colors.
the other stage takes the output of first stage and draws a sky box over it (probably according to depth). Skybox being a large cube surrounding the camera with a "vast landscape" like texture.

Learned how to do stages to a degree.
![second stage drawing the cube with a first stage as color](./images/10.png)

in this i can recognize the dynamic landcape (the first stage that was before), drawn over a cube (second stage), i think. but learned how to pass attachments between stages, skybox should follow.

## how do we create sunrise

Here is what i learnt just now.
There seem to be multiple ways to do lighting. Main consideration is - we need to do a computation linear in number of lights times number of geometry things. We also may have a lot of light sources, and they can be dynamic. Seems we cant just sent everything to the fragment shader and do this the easy way.

So the cool approach everybody are doing is - split the gpu computation into a 2 phases, one does geometry and prepares for the lightning and another does lightning. That can be done in different sequences giving few named methods, like:

- Deferred Lighting (combine all radiances from all lights then do a geometry + lightning).
- Deferred Shading (do all the meshes into the view screen buffer, then do the lightning for each source)
- Light-indexed Deferred Rendering (first pass get indices of lights that are visible then do the geometry with indices)

I however managed to only do the thing Amethyst does in its shaded shader - https://github.com/amethyst/amethyst/blob/main/amethyst_rendy/shaders/fragment/shaded.frag
Not yet sure if albedo counting stuff is the "Deferred Lighting: combine all radiances from all lights", did not have time to figure it out, no albedo currently, and the picture does look not like a sunrise to me. Totally can check the "how do we plop the light-source" box though.

![flying towards the sunrise](./images/8.png)

Now, we can create reflection / diffuse of the sun like this:
https://github.com/jwagner/webglice/blob/master/shaders/sun.glsl

that gives us:

![sun reflection](./images/9.png)

next is the creation of actual sun

## how do we write text on things in 3d

I created a very large texture with all possible texts, like this.
![all texts](./images/6.png). 
Then each number would be in a triangle, like this, ![triangle](./images/7.png).
Thats why there is double space for each text.

Then we properly count the texure coords - key here is that they are in the interval (0, 1).
Then we need to discard non red texture pixels, and we get this:
![red numbers across the scene](./images/5.png)

Thats feels very naive, but works for our purposes.

Bug - each glyph seems to be of different height, so the "-" sign gets to the top of its spot and not to the middle as usual. Will need fixage.

## how do we generate random "cloud like landcapes" on the fly

![screenshot](./images/4.png)

So blender uses some kind of random perlin noise mesh generator, see [add mesh ant landscape](https://github.com/sftd/blender-addons/blob/master/add_mesh_ant_landscape.py) which in turn uses some c written things, see [blendlib intern noise](https://github.com/blender/blender/blob/594f47ecd2d5367ca936cf6fc6ec8168c2b360d0/source/blender/blenlib/intern/noise.c#L1462)
which i had to translate to rust.

then i had to learn how to dynamically supply newly generated landcape meshes from the background thread to the main game loop. pretty neat, but still parts left to do. like, dynamically determine the range for pre-generating terrain based on camera fov area? i.e. generate meshes from here and till the end of visible region. And would be cool to make the generated regions of terrain to be "seamless".

## how do coordinate systems work:
Gltf default camera: “and the default camera sits on the -Z side looking toward the origin with +Y up”

![screenshot](./images/2.png)

(Dimensions of the box (x:1 y:4 z:16)
But:

![screenshot](./images/3.png)

I.e. blender gltf export swaps y and z. Weird.

What is blender convention:
* The /X-axis/ typically represents side-to-side movement.
* The /Y-axis/ represents front-to-back movement.
* The /Z-axis/ goes from top to bottom.

So, `blender gltf exporter` converts coordinate systems: swaps Y and Z. 
If you are designing an asset in blender to be viewable in the default camera in gltf (sitting in -Z:`[0, 0, -1]` pointing towards +Z `[0, 0, 1]` pointing up and with +Y`[0, 1, 0]`

# Attributions

Skybox textures:
https://opengameart.org/content/interstellar-skybox-png
Skybox inspiration:
https://github.com/adrien-ben/gltf-viewer-rs/blob/master/src/renderer/skybox.rs
