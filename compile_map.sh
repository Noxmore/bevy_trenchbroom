#!/bin/bash

# In order to be able to load the textures, we have to be in the textures folder when compiling
cd assets/textures

# Meant to be used with ericw-tools in path
qbsp -bsp2 ../maps/$1.map ../maps/$1.bsp
light -bspx ../maps/$1.bsp
# light -bounce 8 -bspx ../maps/$1.bsp
vis ../maps/$1.bsp

# I like to remove the log files since they are just duplicates of what we get in the terminal
rm ../maps/$1.log
rm light.log
rm vis.log