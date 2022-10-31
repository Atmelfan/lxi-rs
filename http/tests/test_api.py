from http.client import HTTPResponse
import urllib.request

def test_create_ldevid(auth_example, ssl_context):
    request = urllib.request.Request(f"{auth_example[1]}/secure")
    request.add_header("X-API-Key", "ABC.TOKEN")
    response: HTTPResponse = urllib.request.urlopen(request, context=ssl_context)
    assert response.status == 200