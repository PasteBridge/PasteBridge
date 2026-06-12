# -*- coding: utf-8 -*-
import pathlib
# First find where gradle stores the resolved jna-5.13.0.jar
# Look in all gradle cache locations
import os
for root, dirs, files in os.walk(r'C:/Users/Administrator/.gradle'):
    for f in files:
        if f == 'jna-5.13.0.jar':
            p = os.path.join(root, f)
            print(p, os.path.getsize(p))
