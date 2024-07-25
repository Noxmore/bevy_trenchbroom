#!/bin/bash

# Meant to be used with ericw-tools in path
qbsp -bsp2 $1.map
light -bounce 8 $1.bsp
vis $1.bsp