import unittest
from unittest.mock import patch
from urllib.parse import parse_qs

import platform_grant


class PlatformPremiumGrantTests(unittest.TestCase):
    def test_grant_is_submitted_and_approved_in_distinct_sessions(self):
        proposer, approver = object(), object()
        endpoint = "http://127.0.0.1:18080"
        account = "01934f10-7c00-7000-8000-000000000002"
        operation = "01934f10-7c00-7000-8000-000000000003"
        csrf = 'a' * 64
        token = f'<input name="_csrf" value="{csrf}">'
        proposal = (f'<form action="/admin/commerce/grant">{token}'
                    f'<input name="request_id" required value="{operation}"></form>').encode()
        approval_path = f"/admin/commerce/approvals/{operation}"
        approval = f'<form action="{approval_path}">{token}</form>'.encode()
        redirect = (b"", {"Location": "/admin/commerce"})
        with patch.object(platform_grant, "request", side_effect=[
            (proposal, {}), redirect, (approval, {}), redirect,
        ]) as request:
            self.assertEqual(platform_grant.grant_premium(endpoint, account, proposer, approver),
                             operation)
        get_proposal, post_proposal, get_approval, post_approval = request.call_args_list
        self.assertIs(get_proposal.args[0], proposer)
        self.assertIs(get_approval.args[0], approver)
        self.assertEqual(post_proposal.args, (proposer, endpoint + "/admin/commerce/grant"))
        self.assertEqual(parse_qs(post_proposal.kwargs["data"].decode())["target"], [account])
        self.assertEqual(post_approval.args, (approver, endpoint + approval_path))
        self.assertEqual(parse_qs(post_approval.kwargs["data"].decode()), {"_csrf": [csrf]})

    def test_rejects_shared_session_and_nonlocal_platform(self):
        with patch.object(platform_grant, "request") as request:
            owner = object()
            with self.assertRaises(ValueError):
                platform_grant.grant_premium(
                    "http://127.0.0.1:18080", "01934f10-7c00-7000-8000-000000000002",
                    owner, owner,
                )
            with self.assertRaises(RuntimeError):
                platform_grant.grant_premium(
                    "https://example.invalid", "01934f10-7c00-7000-8000-000000000002",
                    owner, object(),
                )
            request.assert_not_called()
