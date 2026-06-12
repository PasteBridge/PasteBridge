
import os, sys, shutil, subprocess, zipfile, glob

# Build paths
mobile_dir = os.path.join(os.getcwd(), 'crates', 'mobile')
apk_debug_dir = os.path.join(mobile_dir, 'androidApp', 'build', 'outputs', 'apk', 'debug')
jna_resources_dir = os.path.join(mobile_dir, 'shared', 'src', 'androidMain', 'resources')
jnilibs_dir = os.path.join(mobile_dir, 'shared', 'src', 'androidMain', 'jniLibs')
gradle_home = os.path.join(os.path.expanduser('~'), '.gradle', 'wrapper', 'dists',
                           'gradle-9.1.0-bin', '9agqghryom9wkf8r80qlhnts3', 'gradle-9.1.0')
gradle_bat = os.path.join(gradle_home, 'bin', 'gradle.bat')

os.chdir(mobile_dir)

# Step 1: Build APK
print('=== Step 1: Building APK ===')
result = subprocess.run([gradle_bat, ':androidApp:assembleDebug', '--no-build-cache', '--rerun-tasks'],
                       capture_output=True, text=True)
with open(os.path.join(mobile_dir, 'build_output.log'), 'w') as f:
    f.write(result.stdout)
    f.write(result.stderr)
print(result.stdout[-2000:] if len(result.stdout) > 2000 else result.stdout)
if result.returncode != 0:
    print('BUILD FAILED:', result.stderr[-1000:])
    sys.exit(1)
print('Build succeeded!')

# Step 2: Find the APK
apks = glob.glob(os.path.join(apk_debug_dir, '*.apk'))
if not apks:
    print('No APK found in', apk_debug_dir)
    sys.exit(1)
apk = max(apks, key=os.path.getmtime)
print('APK:', apk, '(', os.path.getsize(apk), 'bytes)')

# Step 3: Inject JNA android dispatcher resources
print('=== Step 2: Injecting JNA Android dispatchers ===')
so_files = {
    'x86': os.path.join(jnilibs_dir, 'x86', 'libjnidispatch.so'),
    'x86_64': os.path.join(jnilibs_dir, 'x86_64', 'libjnidispatch.so'),
    'arm64-v8a': os.path.join(jnilibs_dir, 'arm64-v8a', 'libjnidispatch.so'),
    'armeabi-v7a': os.path.join(jnilibs_dir, 'armeabi-v7a', 'libjnidispatch.so'),
}
abi_map = {
    'x86': 'x86',
    'x86_64': 'x86_64',
    'arm64-v8a': 'aarch64',
    'armeabi-v7a': 'arm',
}

tmp_apk = apk + '.tmp'
with zipfile.ZipFile(apk, 'r') as zin:
    with zipfile.ZipFile(tmp_apk, 'w', zipfile.ZIP_DEFLATED) as zout:
        for item in zin.infolist():
            data = zin.read(item.filename)
            # Preserve STORE for native libs
            if item.filename.endswith('.so') or item.filename.startswith('lib/'):
                zout.writestr(item, data, compress_type=zipfile.ZIP_STORED)
            else:
                zout.writestr(item, data)
        
        # Add android dispatchers as STORED (resources, not native libs)
        for abi, libpath in so_files.items():
            if os.path.exists(libpath):
                resource_path = 'com/sun/jna/android-' + abi_map[abi] + '/libjnidispatch.so'
                info = zipfile.ZipInfo(resource_path)
                with open(libpath, 'rb') as f:
                    zout.writestr(info, f.read(), compress_type=zipfile.ZIP_STORED)
                print('  Added', resource_path)

os.replace(tmp_apk, apk)
print('APK patched with JNA android dispatchers!')

# Step 4: Re-sign APK
print('=== Step 3: Re-signing APK ===')
# Find apksigner
sdk = os.environ.get('ANDROID_SDK_ROOT', os.path.join(os.environ.get('LOCALAPPDATA', 'C:\\\\Users\\\\Administrator\\\\AppData\\\\Local'), 'Android\\\\Sdk'))
apksigner = None
for root, dirs, files in os.walk(os.path.join(sdk, 'build-tools')):
    for f in files:
        if f == 'apksigner.bat':
            apksigner = os.path.join(root, f)
            break
    if apksigner:
        break

if not apksigner:
    print('WARNING: apksigner not found, trying default path')
    apksigner = os.path.join(sdk, 'build-tools', '37.0.0', 'apksigner.bat')

keystore = os.path.expanduser('~/.android/debug.keystore')
if os.path.exists(keystore):
    result = subprocess.run([apksigner, 'sign', '--ks', keystore, '--ks-pass', 'pass:android',
                            '--ks-key-alias', 'androiddebugkey', '--key-pass', 'pass:android', apk],
                           capture_output=True, text=True)
    print('Sign result:', result.returncode)
    if result.returncode != 0:
        print('STDERR:', result.stderr)
else:
    print('WARNING: debug keystore not found at', keystore)

# Step 5: Verify
print('=== Step 4: Verification ===')
with zipfile.ZipFile(apk, 'r') as z:
    for f in z.namelist():
        if 'android' in f and 'jnidispatch' in f:
            print('  FOUND:', f, '(', z.getinfo(f).file_size, 'bytes)')

result = subprocess.run([apksigner, 'verify', apk], capture_output=True, text=True)
print('Verify:', 'PASS' if result.returncode == 0 else 'FAIL: ' + result.stderr)

print()
print('Done! APK ready for install.')
