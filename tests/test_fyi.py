from fyi_system.fyi import build_prefilled_url, extract_request_id

def test_build_prefilled_url():
    url = build_prefilled_url('auckland_council', 'Hello', 'World', tags=['a:b'])
    assert '/new/auckland_council?' in url
    assert 'title=Hello' in url
    assert 'body=World' in url

def test_extract_request_id():
    assert extract_request_id('https://fyi.org.nz/request/123-test') == 123
    assert extract_request_id('https://fyi.org.nz/search/all') is None
