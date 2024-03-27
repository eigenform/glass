#!/usr/bin/env python3

import numpy as np
import tifffile as tf
import cv2
from sys import argv
from hexdump import hexdump


# CFA pattern: RGGB
# Resolution: 2320x1740
# Bit-depth: 12

if len(argv) < 2: 
    print("{} [raw file]".format(argv[0]))

with open(argv[1], "rb") as f:
    data = f.read()

# Scaled 12-bit to 8-bit RGGB
#arr = np.frombuffer(data, dtype=np.dtype('<u2')) * 16

# 8-bit RGGB
arr = np.frombuffer(data, dtype=np.dtype('<u1'))
#arr.shape = (2320, 1740)
arr.shape = (1740, 2320)
print(arr)

colimg = cv2.cvtColor(arr, cv2.COLOR_BAYER_BGGR2RGBA)
with open("/tmp/wow.rgba8.raw", "wb") as f:
    f.write(colimg.data)

colimg = cv2.cvtColor(arr, cv2.COLOR_BAYER_BGGR2RGB)
with open("/tmp/wow.rgb8.raw", "wb") as f:
    f.write(colimg.data)


cv2.imwrite("/tmp/wow.png", colimg)
