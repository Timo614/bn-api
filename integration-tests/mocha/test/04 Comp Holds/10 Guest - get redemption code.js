const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm');const debug = require("debug");let log=debug('bn-api');
const user = require('../../helpers/user');
const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/redemption_codes/{{last_redemption_code}}';


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


describe('Guest - get redemption code', function () {
    before(async function () {
        await user.registerAndLogin();
        response = await get(requestBody);
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

    it("should be 200", function () {
        expect(response.status).to.equal(200);
    })


    it("data should be present", function () {

        let json = JSON.parse(responseBody);
        expect(json.redemption_code).to.equal(pm.environment.get("last_redemption_code"));
        expect(json.hold_type).to.equal("Comp");

    });


});
