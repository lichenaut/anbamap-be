from flashgeotext.geotext import GeoText

geotext = GeoText()

def get_regions(text):
    result = geotext.extract(input_text=text)
    regions = list(result['countries'].keys())
    return regions