project(
  'xfwl4',
  'rust',
  version: '0.1.0-dev',  # FIXME: pull from Cargo.toml?  or perhaps the opposite?
  license: 'GPL-3.0-or-later',
  meson_version: '>= 0.57.0',
  default_options: [
    'buildtype=debugoptimized',
    'warning_level=2',
  ]
)

gnome = import('gnome')
i18n = import('i18n')

dependency_versions = {
  'glib': '>= 2.72.0',
  'gtk': '>= 3.24.0',
  'libdisplay-info': '>= 0.2.0',
  'libdrm': '>= 2.4.0',
  'libgbm': '>= 25.0.0',
  'libinput': '>= 1.28.0',
  'libpixman': '>= 0.44.0',
  'libseat': '>= 0.9.0',
  'libudev': '>= 257',
  'libxfce4kbd-private': '>= 4.21.4',
  'libxfconf': '>= 4.21.1',
  'libxkbcommon': '>= 0.8.0',
  'rust': '>= 1.88.0',
}

cargo = find_program('cargo', version: dependency_versions['rust'])
rustc = find_program('rustc', version: dependency_versions['rust'])

glib = dependency('glib-2.0', version: dependency_versions['glib'])
gtk = dependency('gtk+-3.0', version: dependency_versions['gtk'])
libxfce4kbd_private = dependency('libxfce4kbd-private-3', version: dependency_versions['libxfce4kbd-private'])
xfconf = dependency('libxfconf-0', version: dependency_versions['libxfconf'])

libdisplay_info = dependency('libdisplay-info', version: dependency_versions['libdisplay-info'])
libdrm = dependency('libdrm', version: dependency_versions['libdrm'])
libgbm = dependency('gbm', version: dependency_versions['libgbm'])
libinput = dependency('libinput', version: dependency_versions['libinput'])
libpixman = dependency('pixman-1', version: dependency_versions['libpixman'])
libseat = dependency('libseat', version: dependency_versions['libseat'])
libudev = dependency('libudev', version: dependency_versions['libudev'])
libxkbcommon = dependency('xkbcommon', version: dependency_versions['libxkbcommon'])

install = find_program('install')

rustflags = []

optimization = get_option('optimization')
if optimization in ['0', '1', '2', '3', 's']
  rustflags += ['-C', 'opt-level=@0@'.format(optimization)]
elif optimization == 'g'
  rustflags += ['-C', 'opt-level=1']
else
  rustflags += ['-C', 'opt-level=2']
endif

if get_option('debug')
  rustflags += ['-C', 'debuginfo=full']
else
  rustflags += ['-C', 'debuginfo=none']
endif

cargo_profile = ''
if get_option('buildtype') in ['plain', 'release']
  cargo_profile = 'release'
else
  cargo_profile = 'debug'
endif

cargo_build_test_options = [
  '--manifest-path', meson.current_source_dir() / 'Cargo.toml',
  '--target-dir', '@0@/xfwl4'.format(meson.current_build_dir()),
  '--no-default-features',
]
if cargo_profile == 'release'
  cargo_build_test_options += '--release'
endif

backends = get_option('backends')
if 'udev' in backends
  cargo_build_test_options += ['-F', 'udev']
endif
if 'winit' in backends
  cargo_build_test_options += ['-F', 'winit']
endif
if 'x11' in backends
  cargo_build_test_options += ['-F', 'x11']
endif

if get_option('egl')
  cargo_build_test_options += ['-F', 'egl']
endif

if get_option('xwayland')
  cargo_build_test_options += ['-F', 'xwayland']
endif

if get_option('debug-rendering')
  cargo_build_test_options += ['-F', 'debug']
endif

env = {
  'RUSTFLAGS': ' '.join(rustflags),
  'PREFIX': get_option('prefix'),
  'BINDIR': get_option('prefix') / get_option('bindir'),
  'DATADIR': get_option('prefix') / get_option('datadir'),
  'PKGDATADIR': get_option('prefix') / get_option('datadir') / 'xfce4' / 'xfwl4',
  'LOCALEDIR': get_option('prefix') / get_option('datadir') / 'locale',
  'PKG_CONFIG_PATH': get_option('prefix') / 'lib64/pkgconfig:' + get_option('prefix') / 'share/pkgconfig' + ':/usr/lib64/pkgconfig:/usr/share/pkgconfig',
  'LD_LIBRARY_PATH': get_option('prefix') / 'lib64',
}

xfwl4 = custom_target(
  'xfwl4',
  output: 'xfwl4',
  env: env,
  command: [cargo, 'build'] + cargo_build_test_options,
  console: true,
  build_by_default: true,
  build_always_stale: true,
)
# Can't use custom_target()'s installation feature, as it will install
# _everything_, including all the rust intermediate build files.
meson.add_install_script(
  install,
  '-m', '0755',
  meson.current_build_dir() / 'xfwl4' / cargo_profile / 'xfwl4',
  get_option('prefix') / get_option('bindir'),
)

if meson.version().version_compare('>= 1.4.0')
  cargo_test_cmd = custom_target(
    'xfwl4-test.sh',
    output: 'xfwl4-test.sh',
    command: [
      'sh',
      '-c',
      'echo "#!/bin/sh" > @OUTPUT@ && echo "@0@" >> @OUTPUT@ && chmod +x @OUTPUT@'.format(' '.join(['cargo', 'test'] + cargo_build_test_options)),
    ],
  )
  test(
    'xfwl4',
    cargo_test_cmd,
    env: env,
    depends: xfwl4,
    protocol: 'exitcode',
  )
endif

subdir('po')
subdir('resources')
