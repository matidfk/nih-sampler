# Nih-Sampler

![Screenshot](./screenshot.png)

A simple sampler written with [nih-plug](https://github.com/robbert-vdh/nih-plug.git).

Run with:

`cargo xtask bundle nih-sampler`

Features:
- Automatically reload and resample all samples when sample rate changes
- Min and max volume, the volume is calculated by mapping velocity


# TODO:
- find better font
- perhaps add features to not have to use multiple instances of the plugin (like the old version)
- add different channel config support

All code is licensed under the [GPLv3](https://www.gnu.org/licenses/gpl-3.0.txt) license.
