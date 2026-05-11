./target/release/gen-orb-mcp publish \
  --binary "<< parameters.binary >>" \
  --asset-name "<< parameters.asset_name >>" \
  <<# parameters.tag >>--tag "<< parameters.tag >>"<</ parameters.tag >> \
  <<# parameters.dry_run >>--dry-run<</ parameters.dry_run >>