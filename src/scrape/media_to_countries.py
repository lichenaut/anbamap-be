from flashgeotext.geotext import GeoText

geotext = GeoText()

def get_countries(text):
    result = geotext.extract(input_text=text)
    countries = list(result['countries'].keys())
    return countries