#!/bin/bash

# In order to be able to load the textures, we have to be in the textures folder when compiling
cd assets/textures

# Meant to be used with ericw-tools in path
qbsp -bsp2 ../maps/$1.map ../maps/$1.bsp
light ../maps/$1.bsp
vis ../maps/$1.bsp

# I like to remove the log files since they are just duplicates of what we get in the terminal
rm ../maps/$1.log
rm ../maps/$1-light.log
rm ../maps/$1-vis.log

# Also this is here, not really sure what it's for
rm ../maps/$1.texinfo.json