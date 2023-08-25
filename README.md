# search-server
A little server to adapt queries from VimiumC to the AzDo repository API

### Example request
GET http://localhost:59991/repos?some*reponame

Will return with a HTML list of links to be quickly navigated, or will forward via a 301 response if only one result is found.
