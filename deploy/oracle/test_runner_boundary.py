"""Exercise the actual workflow shell before its host SSH boundary."""
import os
from pathlib import Path
import subprocess
import tempfile
from textwrap import dedent
import unittest
import zipfile

WORKFLOW = Path(__file__).resolve().parents[2] / '.github/workflows/oracle-operations.yml'
SOURCE_SHA = 'a' * 40
ARTIFACT_URL = 'https://api.github.com/repos/Oteryn/Oteryn-Game/actions/artifacts/789/zip'


def operate_job():
    return WORKFLOW.read_text().split('  operate:\n', 1)[1]


def operate_script():
    script = dedent(operate_job().split('        run: |\n', 1)[1])
    # Start after the identity/socket checks so the test host's facts are irrelevant.
    return script[script.index('if command -v docker'):]


class RunnerBoundary(unittest.TestCase):
    def test_reachable_alternative_docker_endpoint_stops_before_ssh(self):
        script = operate_script()
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            stubs = {'uname': 'echo aarch64', 'id': 'echo 1001',
                     'docker': 'exit "${PROBE_DOCKER_RESULT:-0}"',
                     'ssh': 'touch "$SSH_BOUNDARY_MARKER"'}
            for name, body in stubs.items():
                path = directory / name
                path.write_text('#!/bin/sh\n' + body + '\n')
                path.chmod(0o755)
            environment = dict(os.environ, PATH=str(directory) + ':' + os.environ['PATH'],
                               ACTUAL_RUNNER_NAME='oteryn-oracle-platform',
                               EXPECTED_RUNNER_NAME='oteryn-oracle-platform',
                               OPERATION='status', SOURCE_SHA='',
                               DOCKER_HOST='tcp://unintended-daemon:2375',
                               SSH_BOUNDARY_MARKER=str(directory / 'ssh-called'))
            rejected = subprocess.run(['bash', '-c', script], env=environment, capture_output=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn(b'unexpectedly controls', rejected.stderr)
            self.assertFalse((directory / 'ssh-called').exists())
            environment['PROBE_DOCKER_RESULT'] = '1'
            allowed = subprocess.run(['bash', '-c', script], env=environment, capture_output=True)
            self.assertEqual(allowed.returncode, 0, allowed.stderr)
            self.assertTrue((directory / 'ssh-called').exists())


class ArtifactHandoff(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.zip = self.directory / 'artifact.zip'
        with zipfile.ZipFile(self.zip, 'w') as archive:
            archive.writestr('image.tar', b'\x00\xffimage\r\n')
        stubs = {
            'docker': 'exit 1',
            'curl': '''
                printf '%s\\n' "$@" > "$TEST_DIR/curl-args"
                cat > "$TEST_DIR/curl-headers"
                while [ "$#" -gt 0 ]; do
                    case "$1" in
                        --output) shift; output="$1" ;;
                    esac
                    shift
                done
                cp "$TEST_DIR/artifact.zip" "$output"
                if [ "${OVERSIZE_ZIP:-}" = 1 ]; then
                    truncate -s 1073741825 "$output"
                fi
            ''',
            'ssh': '''
                printf '%s\\n' "$@" > "$TEST_DIR/ssh-args"
                test -z "${GITHUB_TOKEN+x}" || exit 1
                cat > "$TEST_DIR/ssh-stdin"
            ''',
        }
        for name, body in stubs.items():
            path = self.directory / name
            path.write_text('#!/bin/sh\n' + dedent(body) + '\n')
            path.chmod(0o755)

    def run_operation(self, **overrides):
        environment = dict(os.environ, PATH=f'{self.directory}:{os.environ["PATH"]}',
                           TEST_DIR=str(self.directory), TMPDIR=str(self.directory),
                           OPERATION='deploy', SOURCE_SHA=SOURCE_SHA, ARTIFACT_ID='789',
                           GITHUB_RUN_ID='123', GITHUB_RUN_ATTEMPT='2', GITHUB_TOKEN='test-token')
        environment.update(overrides)
        return subprocess.run(['bash', '-euo', 'pipefail', '-c', operate_script()],
                              env=environment, stdin=subprocess.DEVNULL, capture_output=True)

    def test_deploy_hands_ssh_the_command_and_unchanged_artifact_zip(self):
        result = self.run_operation()
        self.assertEqual(result.returncode, 0, result.stderr)
        curl_args = (self.directory / 'curl-args').read_text().splitlines()
        self.assertIn(ARTIFACT_URL, curl_args)
        self.assertIn('@-', curl_args)
        self.assertEqual((self.directory / 'curl-headers').read_text(), 'Authorization: Bearer test-token\n')
        ssh_args = (self.directory / 'ssh-args').read_text().splitlines()
        self.assertEqual(ssh_args[-2:], ['root@oteryn-deploy-host', f'deploy-image {SOURCE_SHA} 123 2 789'])
        self.assertNotIn('test-token', '\n'.join(ssh_args))
        self.assertEqual((self.directory / 'ssh-stdin').read_bytes(), self.zip.read_bytes())

    def test_non_digit_artifact_id_stops_before_download_or_ssh(self):
        result = self.run_operation(ARTIFACT_ID='789x')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.directory / 'curl-args').exists())
        self.assertFalse((self.directory / 'ssh-args').exists())

    def test_bad_sha_stops_before_download_or_ssh(self):
        result = self.run_operation(SOURCE_SHA='a' * 39 + 'g')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.directory / 'curl-args').exists())
        self.assertFalse((self.directory / 'ssh-args').exists())

    def test_oversize_zip_stops_before_ssh(self):
        result = self.run_operation(OVERSIZE_ZIP='1')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b'1 GiB', result.stderr)
        self.assertFalse((self.directory / 'ssh-args').exists())

    def test_status_bypasses_skipped_build_and_calls_ssh_status(self):
        workflow = WORKFLOW.read_text()
        build_job = workflow.split('  build_image:\n', 1)[1].split('  operate:\n', 1)[0]
        self.assertIn("    if: github.ref == 'refs/heads/main' && inputs.action == 'deploy'\n", build_job)
        self.assertIn('    needs: build_image\n', operate_job())
        self.assertIn(
            "    if: always() && github.ref == 'refs/heads/main' && (inputs.action == 'status' || "
            "(inputs.action == 'deploy' && needs.build_image.result == 'success'))\n",
            operate_job(),
        )
        result = self.run_operation(OPERATION='status', SOURCE_SHA='', ARTIFACT_ID='')
        self.assertEqual(result.returncode, 0, result.stderr)
        ssh_args = (self.directory / 'ssh-args').read_text().splitlines()
        self.assertEqual(ssh_args[-2:], ['root@oteryn-deploy-host', 'status'])
        self.assertFalse((self.directory / 'curl-args').exists())


if __name__ == '__main__':
    unittest.main()
