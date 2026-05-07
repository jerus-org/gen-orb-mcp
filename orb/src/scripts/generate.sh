gen-orb-mcp generate \
  --orb-path "<< parameters.orb_path >>" \
  <<# parameters.output >>--output "<< parameters.output >>"<</ parameters.output >> \
  <<# parameters.format >>--format "<< parameters.format >>"<</ parameters.format >> \
  <<# parameters.generate_name >>--name "<< parameters.generate_name >>"<</ parameters.generate_name >> \
  <<# parameters.version >>--version "<< parameters.version >>"<</ parameters.version >> \
  <<# parameters.force >>--force<</ parameters.force >> \
  <<# parameters.migrations >>--migrations "<< parameters.migrations >>"<</ parameters.migrations >> \
  <<# parameters.prior_versions >>--prior-versions "<< parameters.prior_versions >>"<</ parameters.prior_versions >> \
  <<# parameters.tag_prefix >>--tag-prefix "<< parameters.tag_prefix >>"<</ parameters.tag_prefix >>