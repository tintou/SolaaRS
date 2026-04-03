sources = run_command(
  'find', meson.current_source_dir() / 'src', '-name', '*.rs',
  check: true,
).stdout().strip().split('\n')

custom_target('solaars',
  output: 'solaars',
  input: logitech_hidpp_sources + sources,
  command: [
    cargo, 'build',
    '--manifest-path', meson.current_source_dir() / 'Cargo.toml',
    '--target-dir', meson.project_build_root() / 'cargo-target',
    cargo_args,
    '&&',
    'cp',
    meson.project_build_root() / 'cargo-target' / cargo_profile / 'solaars',
    '@OUTPUT@',
  ],
  build_by_default: true,
  install: true,
  install_dir: get_option('bindir'),
)
