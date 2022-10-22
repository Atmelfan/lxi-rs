from http.client import HTTPResponse
import urllib.request
import os
from lxml import etree

def validate(xml: str, xsd_path: str) -> bool:

    xmlschema_doc = etree.parse(xsd_path)
    xmlschema = etree.XMLSchema(xmlschema_doc)

    xml_doc = etree.fromstring(xml)
    result = xmlschema.validate(xml_doc)
    print(xmlschema.error_log)

    return result


def test_identification_xml(http_example, request):
    response: HTTPResponse = urllib.request.urlopen(f"{http_example}/lxi/identification")
    assert response.status == 200
    assert validate(response.read(), os.path.join(request.fspath.dirname, "xsd", "LXIIdentification.xsd"))
