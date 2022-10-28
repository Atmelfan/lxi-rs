import base64
from http.client import HTTPResponse
import urllib.request
import pytest

def test_auth_http(auth_example):
    response: HTTPResponse = urllib.request.urlopen(f"{auth_example[0]}/")
    assert response.status == 200

def test_secure_http(auth_example):
    # Should be redirected to https and then raise an exception without ssl context
    with pytest.raises(urllib.error.URLError) as e_info:
        response: HTTPResponse = urllib.request.urlopen(f"{auth_example[0]}/secure")

def test_auth_https(auth_example, ssl_context):
    response: HTTPResponse = urllib.request.urlopen(f"{auth_example[1]}/", context=ssl_context)
    assert response.status == 200

def test_auth_basic(auth_example, ssl_context):
    request = urllib.request.Request(f"{auth_example[1]}/secure")
    base64string = base64.b64encode(b"Basil:cool meow time").decode()
    print("Basic %s" % base64string)
    request.add_header("Authorization", "Basic %s" % base64string)
    response: HTTPResponse = urllib.request.urlopen(request, context=ssl_context)
    assert response.status == 200

def test_auth_lxiapi(auth_example, ssl_context):
    request = urllib.request.Request(f"{auth_example[1]}/secure")
    request.add_header("X-API-Key", "ABC.TOKEN")
    response: HTTPResponse = urllib.request.urlopen(request, context=ssl_context)
    assert response.status == 200