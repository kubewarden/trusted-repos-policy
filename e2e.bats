#!/usr/bin/env bats

@test "Pod: reject latest tag" {
  run kwctl run \
    --request-path test_data/pod_creation_latest.json \
    --settings-json '{"tags": {"reject": ["latest"]}}'\
    annotated-policy.wasm

  # this prints the output when one the checks below fails
  echo "output = ${output}"

  [ "$status" -eq 0 ]
  [ $(expr "$output" : '.*"allowed":false.*') -ne 0 ]
  [ $(expr "$output" : '.*"message":".*tags not allowed: latest.*') -ne 0 ]
}

@test "Pod: reject implicit latest tag" {
  run kwctl run \
    --request-path test_data/pod_creation_implicit_latest.json \
    --settings-json '{"tags": {"reject": ["latest"]}}'\
    annotated-policy.wasm

  # this prints the output when one the checks below fails
  echo "output = ${output}"

  [ "$status" -eq 0 ]
  [ $(expr "$output" : '.*"allowed":false.*') -ne 0 ]
  [ $(expr "$output" : '.*"message":".*tags not allowed: latest.*') -ne 0 ]
}

@test "CronJob: accept image from allowed registry" {
  run kwctl run \
    --request-path test_data/cronjob_creation.json \
    --settings-json '{"registries": {"allow": ["ghcr.io"]}, "tags": {"reject": ["latest"]}}'\
    annotated-policy.wasm

  # this prints the output when one the checks below fails
  echo "output = ${output}"

  [ "$status" -eq 0 ]
  [ $(expr "$output" : '.*"allowed":true.*') -ne 0 ]
}

@test "DaemonSet: reject not allowed image" {
  run kwctl run \
    --request-path test_data/daemonset_creation.json \
    --settings-json '{"images": {"reject": ["ghcr.io/kubewarden/test-verify-image-signatures:signed"]}}'\
    annotated-policy.wasm

  # this prints the output when one the checks below fails
  echo "output = ${output}"

  [ "$status" -eq 0 ]
  [ $(expr "$output" : '.*"allowed":false.*') -ne 0 ]
  [ $(expr "$output" : '.*"message":".*images not allowed: ghcr.io/kubewarden/test-verify-image-signatures:signed.*') -ne 0 ]
}

@test "Job: accept allowed image" {
  run kwctl run \
    --request-path test_data/job_creation.json \
    --settings-json '{"images": {"allow": ["ghcr.io/kubewarden/test-verify-image-signatures:signed"]}}'\
    annotated-policy.wasm

  # this prints the output when one the checks below fails
  echo "output = ${output}"

  [ "$status" -eq 0 ]
  [ $(expr "$output" : '.*"allowed":true.*') -ne 0 ]
}

@test "Job: settings not valid because of tag " {
  run kwctl run \
    --request-path test_data/job_creation.json \
    --settings-json '{"images": {"allow": ["not-a-valid-image-tag:1.0.0+rc1"]}}'\
    annotated-policy.wasm

  # this prints the output when one the checks below fails
  echo "output = ${output}"

  [ "$status" -eq 1 ]
  [ $(expr "$output" : '.*Provided settings are not valid.*invalid reference format.*') -ne 0 ]
}
