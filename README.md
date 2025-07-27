# wine-api-saas
An example on how you can build an MVP SaaS in a day


## API Endpoints

Core Data Access
GET /wines

Purpose: Get all wines with optional filtering
Example Query Parameters:

region=California
variety=Red Wine
min_rating=90
max_rating=95


Aggregated Data (No Complex Joins)
GET /regions

Purpose: List all unique regions with wine counts
Response: {"Ribera del Duero, Spain": 2, "California": 5, "Mendocino, California": 2}
Implementation: Simple GROUP BY on region column

GET /varieties

Purpose: List all wine varieties with counts and avg ratings
Response: {"Red Wine": {"count": 9, "avg_rating": 91.2}}

Search & Discovery
GET /wines/search?q=bourbon

Purpose: Search wine names and notes for keywords
Implementation: Simple LIKE/ILIKE query on name and notes columns

GET /wines/region/{region}

Purpose: Get wines from specific region
Example: /wines/region/California
URL-encode spaces: /wines/region/Ribera%20del%20Duero,%20Spain
