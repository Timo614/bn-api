const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm');const debug = require("debug");var log=debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/cart';


var response;
var responseBody;


const put = async function (request_body) {
    return baseUrl
        .put(pm.substitute(apiEndPoint))
        .set('Accept', 'application/json')
        .set('Content-Type', 'application/json')
        .set('Authorization', pm.substitute('Bearer {{discount_user_token}}'))

        .send(pm.substitute(request_body));
};

const get = async function (request_body) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))

        .set('Authorization', pm.substitute('Bearer {{discount_user_token}}'))

        .set('Accept', 'application/json')
        .send();
};

let requestBody = `{
"items": [{
"ticket_type_id": "{{vip_ticket_type_id}}",
"quantity":11,
"redemption_code": "{{discount_percentage_redemption_code}}"
}
]}`;


describe('User - add more than max discount uses to cart', function () {
    before(async function () {
        response = await put(requestBody);
        log(response.request.header);
        log(response.request.url);
        log(response.request._data);
        log(response.request.method);
        responseBody = JSON.stringify(response.body);
        //log(pm);
        log(response.status);
        log(responseBody);
    });

    after(async function () {
        // add after methods


    });

    it("should be 422", function () {
        expect(response.status).to.equal(422);
    })


    it("Error code should be max_uses_reached", function () {


        let json = JSON.parse(responseBody);

        pm.environment.set("last_cart_id", json.cart_id);
        expect(json.fields.quantity[0].code).to.equal("max_uses_reached");

    });


});
