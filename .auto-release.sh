#!/bin/bash

repo_slug="vault713/wallet713"
token="$GITHUB_OAUTH_TOKEN"
export CHANGELOG_GITHUB_TOKEN="$token"

tagname=`git describe --tags --exact-match 2>/dev/null || git symbolic-ref -q --short HEAD`

echo 'make a tarball for the release binary...\n'

if [[ $TRAVIS_OS_NAME == 'osx' ]]; then
    cd target/release ; rm -f *.tgz; tar zcf "wallet713-$tagname-$TRAVIS_JOB_ID-osx.tgz" wallet713
    /bin/ls -ls *.tgz  | awk '{print $6,$7,$8,$9,$10}'
    md5 "wallet713-$tagname-$TRAVIS_JOB_ID-osx.tgz" > "wallet713-$tagname-$TRAVIS_JOB_ID-osx.tgz"-md5sum.txt
    /bin/ls -ls *-md5sum.txt  | awk '{print $6,$7,$8,$9,$10}'
    cd - > /dev/null;
    echo "tarball generated\n"
    # Only generate changelog on Linux platform, to avoid duplication
    exit 0
else
    # Do some custom requirements on Linux
    cd target/release ; rm -f *.tgz; tar zcf "wallet713-$tagname-$TRAVIS_JOB_ID-linux-amd64.tgz" wallet713
    /bin/ls -ls *.tgz  | awk '{print $6,$7,$8,$9,$10}'
    md5sum "wallet713-$tagname-$TRAVIS_JOB_ID-linux-amd64.tgz" > "wallet713-$tagname-$TRAVIS_JOB_ID-linux-amd64.tgz"-md5sum.txt
    /bin/ls -ls *-md5sum.txt  | awk '{print $6,$7,$8,$9,$10}'
    cd - > /dev/null;
    echo "tarball generated\n"
fi

version="$tagname"
branch="`git symbolic-ref -q --short HEAD`"

# automatic changelog generator
gem install github_changelog_generator

LAST_REVISION=$(git rev-list --tags --skip=1 --max-count=1)
LAST_RELEASE_TAG=$(git describe --abbrev=0 --tags ${LAST_REVISION})

# Generate CHANGELOG.md
github_changelog_generator \
  -u $(cut -d "/" -f1 <<< $repo_slug) \
  -p $(cut -d "/" -f2 <<< $repo_slug) \
  -t $token \
  --since-tag ${LAST_RELEASE_TAG}

body="$(cat CHANGELOG.md)"

# Overwrite CHANGELOG.md with JSON data for GitHub API
jq -n \
  --arg body "$body" \
  --arg name "$version" \
  --arg tag_name "$version" \
  --arg target_commitish "$branch" \
  '{
    body: $body,
    name: $name,
    tag_name: $tag_name,
    target_commitish: $target_commitish,
    draft: false,
    prerelease: false
  }' > CHANGELOG.md

release_id="$(curl -0 -XGET -H "Authorization: token $token" https://api.github.com/repos/$repo_slug/releases/tags/$tagname 2>/dev/null | grep id | head -n 1 | sed 's/ *"id": *\(.*\),/\1/')"
echo "Updating release $version for repo: $repo_slug, branch: $branch. release id: $release_id"
curl -H "Authorization: token $token" --request PATCH  --data @CHANGELOG.md "https://api.github.com/repos/$repo_slug/releases/$release_id"
echo "auto changelog uploaded.\n"
