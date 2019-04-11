const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm')

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/events/{{last_event_id}}?redemption_code={{access_redemption_code}}';


var response;
var responseBody;


const post = async function (request_body) {
    return baseUrl
        .post(pm.substitute(apiEndPoint))
        .set('Accept', 'application/json')
        .set('Content-Type', 'application/json')

        .send(pm.substitute(request_body));
};

const get = async function (request_body) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))


        .set('Accept', 'application/json')
        .send();
};

let requestBody = ``;
let r = {};
let access_code = {};

describe('Guest - view event tickets with access code', function () {
    before(async function () {
        response = await get(requestBody);
        console.log(response.request.header);
        console.log(response.request.url);
        console.log(response.request._data);
        console.log(response.request.method);
        responseBody = JSON.stringify(response.body);
        //console.log(pm);
        console.log(response.status);
        console.log(responseBody);

        r = JSON.parse(responseBody);

        access_code = pm.environment.get("access_redemption_code");
    });

    after(async function () {
        // add after methods


    });

    it("should be 200", function () {
        expect(response.status).to.equal(200);
    });

    it("should have access gated ticket type", function () {
        expect(r.ticket_types.length).to.equal(4);
        expect(r.ticket_types[3].redemption_code).to.equal(access_code);
    });

    it("should have normal pricing", function () {
        expect(r.ticket_types[0].ticket_pricing.price_in_cents).to.equal(3000);
        expect(r.ticket_types[0].ticket_pricing.discount_in_cents).to.equal(0);
    });

    it("should return event fee_in_cents", function () {
        expect(r.fee_in_cents).to.equal(100);
    });

    it("should have correct ticket_types ticket_pricing fee_in_cents", function () {
        expect(r.ticket_types[0].ticket_pricing.fee_in_cents).to.equal(10);
    });


});

            
