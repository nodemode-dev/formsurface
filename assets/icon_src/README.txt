formsurface app icon: the owner's planar bust (bust_source.png, the same render formsculptor uses),
half left in its original white clay and half spray-painted graffiti-style in the Form family gradient:
a sprayed crown, a tag sweep across the face with drips, a burst on the plinth, and overspray grain.

paint4.py  upscales the bust 4x (EDSR via opencv dnn_superres; download EDSR_x4.pb from
           github.com/Saafke/EDSR_Tensorflow), smooths the cutout's edge, and paints it
compose.py places it on the family tile (bloom, rim, drop shadow) for the macOS and full-bleed sets
Every icon file is a resize of those two 1024 renders.
