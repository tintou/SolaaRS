sources = run_command(
  'find', meson.current_source_dir() / 'src', '-name', '*.rs',
  check: true,
).stdout().strip().split('\n')

custom_target('solaarsd',
  output: 'solaarsd',
  input: logitech_hidpp_sources + sources,
  command: [
    cargo, 'build',
    '--manifest-path', meson.current_source_dir() / 'Cargo.toml',
    '--target-dir', meson.project_build_root() / 'cargo-target',
    cargo_args,
    '&&',
    'cp',
    meson.project_build_root() / 'cargo-target' / cargo_profile / 'solaarsd',
    '@OUTPUT@',
  ],
  build_by_default: true,
  install: true,
  install_dir: get_option('sbindir'),
)

# ── D-Bus interface XML ───────────────────────────────────────────────────────

dbus_data_dir = get_option('prefix') / get_option('datadir') / 'dbus-1'

install_data(
  meson.current_source_dir() / 'data' / 'org.solaarsd.xml',
  install_dir: dbus_data_dir / 'interfaces',
)

# ── D-Bus system service activation ──────────────────────────────────────────

install_data(
  'data' / 'org.solaarsd.service',
  install_dir: dbus_data_dir / 'system-services',
)

install_data(
  'data' / 'org.solaarsd.conf',
  install_dir: get_option('sysconfdir') / 'dbus-1' / 'system.d',
)

# ── systemd unit ──────────────────────────────────────────────────────────────

dbus_service_conf = configuration_data()
dbus_service_conf.set('sbindir', get_option('prefix') / get_option('sbindir'))

systemd = dependency('systemd', required: false)
systemd_unit_dir = systemd.found() \
  ? systemd.get_variable('systemdsystemunitdir') \
  : get_option('prefix') / 'lib' / 'systemd' / 'system'

configure_file(
  input: 'data' / 'solaarsd.service.in',
  output: 'solaarsd.service',
  configuration: dbus_service_conf,
  install: true,
  install_dir: systemd_unit_dir,
)
