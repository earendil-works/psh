cd /work/oils
mkdir -p /work/results
run1() {
  name=$1
  PYTHONPATH=. python3 test/sh_spec3.py --timeout 10 --tmp-env /tmp/spec/$name --path-env "/work/oils/spec/bin:$PATH" \
    --env-pair LC_ALL=C.UTF-8 --env-pair REPO_ROOT=/work/oils \
    spec/$name.test.sh bash dash mksh zsh /work/shells/ash /work/shells/brush /work/shells/gosh /work/shells/yash3 /work/shells/bunsh /work/shells/deno-task \
    > /work/results/$name.txt 2>&1
  echo done $name
}
export -f run1
cat /work/SUITE-compat.txt | xargs -P 8 -I{} bash -c 'run1 {}'
