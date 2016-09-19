#!/usr/bin/env ruby

def perror(msg, warn_only=false)
    if warn_only then
        puts "INFO: #{msg}"
        exit 2
    end
    STDERR.puts "ERROR: #{msg}" 
    exit 1
end

def get_toml_version
    contents = File.read("Cargo.toml")
    version = contents.match("version\s*=\s*\"(.*)\"")
    if version.nil?
        perror("Couldn't get project version from Cargo.toml")
    else
        version[1]
    end
end

arch=ENV['TRAVIS_OS_NAME']
tag=ENV['TRAVIS_TAG']

rust_version=ENV['TRAVIS_RUST_VERSION']

if arch.nil?
    perror "Operating system is unknown (TRAVIS_OS_NAME not set)"
end

if tag.nil?
    perror "Tag version is unknown (TRAVIS_TAG not set)"
end

if rust_version.nil?
    perror "Rust version is unknown (TRAVIS_RUST_VERSION not set)"
end

if rust_version != "stable"
    perror "Not deploying from non-stable Rust", true
end

bintray_user=ENV['BINTRAY_SNOWPLOW_GENERIC_USER']
bintray_key=ENV['BINTRAY_SNOWPLOW_GENERIC_API_KEY']

if bintray_user.nil?
    perror "Bintray user is unknown (BINTRAY_SNOWPLOW_GENERIC_USER is not set)"
end

if bintray_key.nil?
    perror "Bintray key is unknown (BINTRAY_SNOWPLOW_GENERIC_API_KEY is not set)"
end

if tag.match(/^\d+\.\d+\.\d+-?.*$/).nil?
    perror "Ignoring tag '#{tag}' as it isn't a deployable version", true
end

toml_version = get_toml_version
if tag != toml_version
    perror "Tag '#{tag}' does not match the version in Cargo.toml ('#{toml_version}')"
end
