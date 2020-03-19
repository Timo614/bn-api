const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm');const debug = require("debug");var log=debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/events/{{last_event_id}}/availability?box_office_pricing=true';

var response;
var responseBody;

const post = async function (request_body) {
    return baseUrl
        .post(pm.substitute(apiEndPoint))
        .set('Accept', 'application/json')
        .set('Content-Type', 'application/json')
        .set('Authorization', pm.substitute('Bearer {{org_boxoffice_token}}'))

        .send(pm.substitute(request_body));
};

const get = async function (request_body) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))

        .set('Authorization', pm.substitute('Bearer {{org_boxoffice_token}}'))

        .set('Accept', 'application/json')
        .send();
};

let requestBody = ``;

let r = {};
describe('Box Office - View Event Tickets', function () {
    before(async function () {
        response = await get(requestBody);
        log(response.request.header);
        log(response.request.url);
        log(response.request._data);
        log(response.request.method);
        responseBody = JSON.stringify(response.body, null, 2);
        //log(pm);
        log(response.status);
        log(responseBody);

        r = JSON.parse(responseBody);
    });

    it("should be 200", function () {
        expect(response.status).to.equal(200);
    });

    it("should have box office pricing", function () {
        expect(r.ticket_types[3].ticket_pricing.price_in_cents).to.equal(4000);
    });

    it("should have correct ticket_types ticket_pricing fee_in_cents", function () {
        expect(r.ticket_types[3].ticket_pricing.fee_in_cents).to.equal(0);
    });
});
