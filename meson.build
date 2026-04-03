project('solaar-rs', version: '0.1.0',
  license: 'GPL-2.0-or-later',
  meson_version: '>= 1.0',
)

cargo = find_program('cargo', required: true)

if get_option('buildtype') == 'debug'
  cargo_profile = 'debug'
  cargo_args = []
else
  cargo_profile = 'release'
  cargo_args = ['--release']
endif

# Shared library sources — changes here rebuild both binaries.
logitech_hidpp_sources = run_command(
  'find', meson.project_source_root() / 'logitech-hidpp' / 'src', '-name', '*.rs',
  check: true,
).stdout().strip().split('\n')

subdir('solaars-cli')
subdir('solaarsd')

# ── udev rules ────────────────────────────────────────────────────────────────

udev = dependency('udev', required: false)
udev_rules_dir = udev.found() \
  ? udev.get_variable('udevdir') / 'rules.d' \
  : get_option('prefix') / 'lib' / 'udev' / 'rules.d'

install_data(
  'rules.d' / '42-logitech-unify-permissions.rules',
  install_dir: udev_rules_dir,
)
