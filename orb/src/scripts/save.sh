./target/release/gen-orb-mcp save \
  --paths "<< parameters.paths >>" \
  <<# parameters.message >>--message "<< parameters.message >>"<</ parameters.message >> \
  <<# parameters.push >>--push "<< parameters.push >>"<</ parameters.push >> \
  <<# parameters.no_push >>--no-push<</ parameters.no_push >> \
  <<# parameters.dry_run >>--dry-run<</ parameters.dry_run >>