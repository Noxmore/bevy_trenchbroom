#!/bin/bash

set -e

# Meant to be used with ericw-tools in path
qbsp -bsp2 -wrbrushesonly -nosubdivide -nosoftware -path assets -notex assets/maps/$1.map assets/maps/$1.bsp
light -extra4 -novanilla -lightgrid -world_units_per_luxel 4 -path assets assets/maps/$1.bsp
# NOTE: -world_units_per_luxel 4 is quite high, use something more reasonable on larger maps.

# I like to remove the log files since they are just duplicates of what we get in the terminal
rm assets/maps/$1.log
rm assets/maps/$1-light.log

# Contains phong information, which currently we don't use.
rm assets/maps/$1.texinfo.json