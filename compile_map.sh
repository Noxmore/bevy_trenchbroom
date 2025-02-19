#!/bin/bash

set -e

# Meant to be used with ericw-tools in path
qbsp -bsp2 -wrbrushesonly -nosubdivide -nosoftware -path assets/textures assets/maps/$1.map assets/maps/$1.bsp
light -extra4 -novanilla -lightgrid assets/maps/$1.bsp
vis assets/maps/$1.bsp

# I like to remove the log files since they are just duplicates of what we get in the terminal
rm assets/maps/$1.log
rm assets/maps/$1-light.log
rm assets/maps/$1-vis.log

# Contains phong information, which currently we don't use.
rm assets/maps/$1.texinfo.json