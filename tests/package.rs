#[macro_use]
extern crate balertest;
extern crate flate2;
extern crate git2;
extern crate hamcrest;
extern crate tar;
extern crate baler;

use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use balertest::{baler_process, process};
use balertest::support::{project, execs, paths, git, path2url, baler_exe};
use balertest::support::registry::Package;
use flate2::read::GzDecoder;
use hamcrest::{assert_that, existing_file, contains, equal_to};
use tar::Archive;

#[test]
fn simple() {
    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
            exclude = ["*.txt"]
            license = "MIT"
            description = "foo"
        "#)
        .file("src/main.rs", r#"
            fn main() { println!("hello"); }
        "#)
        .file("src/bar.txt", ""); // should be ignored when packaging

    assert_that(p.baler_process("package"),
                execs().with_status(0).with_stderr(&format!("\
[WARNING] manifest has no documentation[..]
See [..]
[PACKAGING] foo v0.0.1 ({dir})
[VERIFYING] foo v0.0.1 ({dir})
[COMPILING] foo v0.0.1 ({dir}[..])
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        dir = p.url())));
    assert_that(&p.root().join("target/package/foo-0.0.1.crate"), existing_file());
    assert_that(p.baler("package").arg("-l"),
                execs().with_status(0).with_stdout("\
Baler.toml
src[/]main.rs
"));
    assert_that(p.baler("package"),
                execs().with_status(0).with_stdout(""));

    let f = File::open(&p.root().join("target/package/foo-0.0.1.crate")).unwrap();
    let mut rdr = GzDecoder::new(f).unwrap();
    let mut contents = Vec::new();
    rdr.read_to_end(&mut contents).unwrap();
    let mut ar = Archive::new(&contents[..]);
    for f in ar.entries().unwrap() {
        let f = f.unwrap();
        let fname = f.header().path_bytes();
        let fname = &*fname;
        assert!(fname == b"foo-0.0.1/Baler.toml" ||
                fname == b"foo-0.0.1/Baler.toml.orig" ||
                fname == b"foo-0.0.1/src/main.rs",
                "unexpected filename: {:?}", f.header().path())
    }
}

#[test]
fn metadata_warning() {
    let p = project("all")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#);
    assert_that(p.baler_process("package"),
                execs().with_status(0).with_stderr(&format!("\
warning: manifest has no description, license, license-file, documentation, \
homepage or repository.
See http://doc.crates.io/manifest.html#package-metadata for more info.
[PACKAGING] foo v0.0.1 ({dir})
[VERIFYING] foo v0.0.1 ({dir})
[COMPILING] foo v0.0.1 ({dir}[..])
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        dir = p.url())));

    let p = project("one")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
            license = "MIT"
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#);
    assert_that(p.baler_process("package"),
                execs().with_status(0).with_stderr(&format!("\
warning: manifest has no description, documentation, homepage or repository.
See http://doc.crates.io/manifest.html#package-metadata for more info.
[PACKAGING] foo v0.0.1 ({dir})
[VERIFYING] foo v0.0.1 ({dir})
[COMPILING] foo v0.0.1 ({dir}[..])
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        dir = p.url())));

    let p = project("all")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
            license = "MIT"
            description = "foo"
            repository = "bar"
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#);
    assert_that(p.baler_process("package"),
                execs().with_status(0).with_stderr(&format!("\
[PACKAGING] foo v0.0.1 ({dir})
[VERIFYING] foo v0.0.1 ({dir})
[COMPILING] foo v0.0.1 ({dir}[..])
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        dir = p.url())));
}

#[test]
fn package_verbose() {
    let root = paths::root().join("all");
    let p = git::repo(&root)
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#)
        .file("a/Baler.toml", r#"
            [project]
            name = "a"
            version = "0.0.1"
            authors = []
        "#)
        .file("a/src/lib.rs", "");
    p.build();
    let mut baler = baler_process();
    baler.cwd(p.root());
    assert_that(baler.clone().arg("build"), execs().with_status(0));

    println!("package main repo");
    assert_that(baler.clone().arg("package").arg("-v").arg("--no-verify"),
                execs().with_status(0).with_stderr("\
[WARNING] manifest has no description[..]
See http://doc.crates.io/manifest.html#package-metadata for more info.
[PACKAGING] foo v0.0.1 ([..])
[ARCHIVING] [..]
[ARCHIVING] [..]
"));

    println!("package sub-repo");
    assert_that(baler.arg("package").arg("-v").arg("--no-verify")
                     .cwd(p.root().join("a")),
                execs().with_status(0).with_stderr("\
[WARNING] manifest has no description[..]
See http://doc.crates.io/manifest.html#package-metadata for more info.
[PACKAGING] a v0.0.1 ([..])
[ARCHIVING] [..]
[ARCHIVING] [..]
"));
}

#[test]
fn package_verification() {
    let p = project("all")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#);
    assert_that(p.baler_process("build"),
                execs().with_status(0));
    assert_that(p.baler("package"),
                execs().with_status(0).with_stderr(&format!("\
[WARNING] manifest has no description[..]
See http://doc.crates.io/manifest.html#package-metadata for more info.
[PACKAGING] foo v0.0.1 ({dir})
[VERIFYING] foo v0.0.1 ({dir})
[COMPILING] foo v0.0.1 ({dir}[..])
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        dir = p.url())));
}

#[test]
fn path_dependency_no_version() {
    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
            license = "MIT"
            description = "foo"

            [dependencies.bar]
            path = "bar"
        "#)
        .file("src/main.rs", "fn main() {}")
        .file("bar/Baler.toml", r#"
            [package]
            name = "bar"
            version = "0.0.1"
            authors = []
        "#)
        .file("bar/src/lib.rs", "");

    assert_that(p.baler_process("package"),
                execs().with_status(101).with_stderr("\
[WARNING] manifest has no documentation, homepage or repository.
See http://doc.crates.io/manifest.html#package-metadata for more info.
[ERROR] all path dependencies must have a version specified when packaging.
dependency `bar` does not specify a version.
"));
}

#[test]
fn exclude() {
    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
            exclude = [
                "*.txt",
                # file in root
                "file_root_1",       # NO_CHANGE (ignored)
                "/file_root_2",      # CHANGING (packaged -> ignored)
                "file_root_3/",      # NO_CHANGE (packaged)
                "file_root_4/*",     # NO_CHANGE (packaged)
                "file_root_5/**",    # NO_CHANGE (packaged)
                # file in sub-dir
                "file_deep_1",       # CHANGING (packaged -> ignored)
                "/file_deep_2",      # NO_CHANGE (packaged)
                "file_deep_3/",      # NO_CHANGE (packaged)
                "file_deep_4/*",     # NO_CHANGE (packaged)
                "file_deep_5/**",    # NO_CHANGE (packaged)
                # dir in root
                "dir_root_1",        # CHANGING (packaged -> ignored)
                "/dir_root_2",       # CHANGING (packaged -> ignored)
                "dir_root_3/",       # CHANGING (packaged -> ignored)
                "dir_root_4/*",      # NO_CHANGE (ignored)
                "dir_root_5/**",     # NO_CHANGE (ignored)
                # dir in sub-dir
                "dir_deep_1",        # CHANGING (packaged -> ignored)
                "/dir_deep_2",       # NO_CHANGE
                "dir_deep_3/",       # CHANGING (packaged -> ignored)
                "dir_deep_4/*",      # CHANGING (packaged -> ignored)
                "dir_deep_5/**",     # CHANGING (packaged -> ignored)
            ]
        "#)
        .file("src/main.rs", r#"
            fn main() { println!("hello"); }
        "#)
        .file("bar.txt", "")
        .file("src/bar.txt", "")
        // file in root
        .file("file_root_1", "")
        .file("file_root_2", "")
        .file("file_root_3", "")
        .file("file_root_4", "")
        .file("file_root_5", "")
        // file in sub-dir
        .file("some_dir/file_deep_1", "")
        .file("some_dir/file_deep_2", "")
        .file("some_dir/file_deep_3", "")
        .file("some_dir/file_deep_4", "")
        .file("some_dir/file_deep_5", "")
        // dir in root
        .file("dir_root_1/some_dir/file", "")
        .file("dir_root_2/some_dir/file", "")
        .file("dir_root_3/some_dir/file", "")
        .file("dir_root_4/some_dir/file", "")
        .file("dir_root_5/some_dir/file", "")
        // dir in sub-dir
        .file("some_dir/dir_deep_1/some_dir/file", "")
        .file("some_dir/dir_deep_2/some_dir/file", "")
        .file("some_dir/dir_deep_3/some_dir/file", "")
        .file("some_dir/dir_deep_4/some_dir/file", "")
        .file("some_dir/dir_deep_5/some_dir/file", "")
        ;

    assert_that(p.baler_process("package").arg("--no-verify").arg("-v"),
                execs().with_status(0).with_stdout("").with_stderr("\
[WARNING] manifest has no description[..]
See http://doc.crates.io/manifest.html#package-metadata for more info.
[PACKAGING] foo v0.0.1 ([..])
[WARNING] [..] file `dir_root_1[/]some_dir[/]file` WILL be excluded [..]
See [..]
[WARNING] [..] file `dir_root_2[/]some_dir[/]file` WILL be excluded [..]
See [..]
[WARNING] [..] file `dir_root_3[/]some_dir[/]file` WILL be excluded [..]
See [..]
[WARNING] [..] file `file_root_2` WILL be excluded [..]
See [..]
[WARNING] [..] file `some_dir[/]dir_deep_1[/]some_dir[/]file` WILL be excluded [..]
See [..]
[WARNING] [..] file `some_dir[/]dir_deep_3[/]some_dir[/]file` WILL be excluded [..]
See [..]
[WARNING] [..] file `some_dir[/]dir_deep_4[/]some_dir[/]file` WILL be excluded [..]
See [..]
[WARNING] [..] file `some_dir[/]dir_deep_5[/]some_dir[/]file` WILL be excluded [..]
See [..]
[WARNING] [..] file `some_dir[/]file_deep_1` WILL be excluded [..]
See [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
"));

    assert_that(&p.root().join("target/package/foo-0.0.1.crate"), existing_file());

    assert_that(p.baler("package").arg("-l"),
                execs().with_status(0).with_stdout("\
Baler.toml
dir_root_1[/]some_dir[/]file
dir_root_2[/]some_dir[/]file
dir_root_3[/]some_dir[/]file
file_root_2
file_root_3
file_root_4
file_root_5
some_dir[/]dir_deep_1[/]some_dir[/]file
some_dir[/]dir_deep_2[/]some_dir[/]file
some_dir[/]dir_deep_3[/]some_dir[/]file
some_dir[/]dir_deep_4[/]some_dir[/]file
some_dir[/]dir_deep_5[/]some_dir[/]file
some_dir[/]file_deep_1
some_dir[/]file_deep_2
some_dir[/]file_deep_3
some_dir[/]file_deep_4
some_dir[/]file_deep_5
src[/]main.rs
"));
}

#[test]
fn include() {
    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
            exclude = ["*.txt"]
            include = ["foo.txt", "**/*.rs", "Baler.toml"]
        "#)
        .file("foo.txt", "")
        .file("src/main.rs", r#"
            fn main() { println!("hello"); }
        "#)
        .file("src/bar.txt", ""); // should be ignored when packaging

    assert_that(p.baler_process("package").arg("--no-verify").arg("-v"),
                execs().with_status(0).with_stderr("\
[WARNING] manifest has no description[..]
See http://doc.crates.io/manifest.html#package-metadata for more info.
[PACKAGING] foo v0.0.1 ([..])
[ARCHIVING] [..]
[ARCHIVING] [..]
[ARCHIVING] [..]
"));
}

#[test]
fn package_lib_with_bin() {
    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
        "#)
        .file("src/main.rs", r#"
            extern crate foo;
            fn main() {}
        "#)
        .file("src/lib.rs", "");

    assert_that(p.baler_process("package").arg("-v"),
                execs().with_status(0));
}

#[test]
fn package_git_submodule() {
    let project = git::new("foo", |project| {
        project.file("Baler.toml", r#"
                    [project]
                    name = "foo"
                    version = "0.0.1"
                    authors = ["foo@example.com"]
                    license = "MIT"
                    description = "foo"
                    repository = "foo"
                "#)
                .file("src/lib.rs", "pub fn foo() {}")
    }).unwrap();
    let library = git::new("bar", |library| {
        library.file("Makefile", "all:")
    }).unwrap();

    let repository = git2::Repository::open(&project.root()).unwrap();
    let url = path2url(library.root()).to_string();
    git::add_submodule(&repository, &url, Path::new("bar"));
    git::commit(&repository);

    let repository = git2::Repository::open(&project.root().join("bar")).unwrap();
    repository.reset(&repository.revparse_single("HEAD").unwrap(),
                     git2::ResetType::Hard, None).unwrap();

    assert_that(baler_process().arg("package").cwd(project.root())
                 .arg("--no-verify").arg("-v"),
                execs().with_status(0).with_stderr_contains("[ARCHIVING] bar/Makefile"));
}

#[test]
fn no_duplicates_from_modified_tracked_files() {
    let root = paths::root().join("all");
    let p = git::repo(&root)
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
        "#)
        .file("src/main.rs", r#"
            fn main() {}
        "#);
    p.build();
    File::create(p.root().join("src/main.rs")).unwrap().write_all(br#"
            fn main() { println!("A change!"); }
        "#).unwrap();
    let mut baler = baler_process();
    baler.cwd(p.root());
    assert_that(baler.clone().arg("build"), execs().with_status(0));
    assert_that(baler.arg("package").arg("--list"),
                execs().with_status(0).with_stdout("\
Baler.toml
src/main.rs
"));
}

#[test]
fn ignore_nested() {
    let baler_toml = r#"
            [project]
            name = "nested"
            version = "0.0.1"
            authors = []
            license = "MIT"
            description = "nested"
        "#;
    let main_rs = r#"
            fn main() { println!("hello"); }
        "#;
    let p = project("nested")
        .file("Baler.toml", baler_toml)
        .file("src/main.rs", main_rs)
        // If a project happens to contain a copy of itself, we should
        // ignore it.
        .file("a_dir/nested/Baler.toml", baler_toml)
        .file("a_dir/nested/src/main.rs", main_rs);

    assert_that(p.baler_process("package"),
                execs().with_status(0).with_stderr(&format!("\
[WARNING] manifest has no documentation[..]
See http://doc.crates.io/manifest.html#package-metadata for more info.
[PACKAGING] nested v0.0.1 ({dir})
[VERIFYING] nested v0.0.1 ({dir})
[COMPILING] nested v0.0.1 ({dir}[..])
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        dir = p.url())));
    assert_that(&p.root().join("target/package/nested-0.0.1.crate"), existing_file());
    assert_that(p.baler("package").arg("-l"),
                execs().with_status(0).with_stdout("\
Baler.toml
src[..]main.rs
"));
    assert_that(p.baler("package"),
                execs().with_status(0).with_stdout(""));

    let f = File::open(&p.root().join("target/package/nested-0.0.1.crate")).unwrap();
    let mut rdr = GzDecoder::new(f).unwrap();
    let mut contents = Vec::new();
    rdr.read_to_end(&mut contents).unwrap();
    let mut ar = Archive::new(&contents[..]);
    for f in ar.entries().unwrap() {
        let f = f.unwrap();
        let fname = f.header().path_bytes();
        let fname = &*fname;
        assert!(fname == b"nested-0.0.1/Baler.toml" ||
                fname == b"nested-0.0.1/Baler.toml.orig" ||
                fname == b"nested-0.0.1/src/main.rs",
                "unexpected filename: {:?}", f.header().path())
    }
}

#[cfg(unix)] // windows doesn't allow these characters in filenames
#[test]
fn package_weird_characters() {
    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
        "#)
        .file("src/main.rs", r#"
            fn main() { println!("hello"); }
        "#)
        .file("src/:foo", "");

    assert_that(p.baler_process("package"),
                execs().with_status(101).with_stderr("\
warning: [..]
See [..]
[PACKAGING] foo [..]
[ERROR] failed to prepare local package for uploading

Caused by:
  cannot package a filename with a special character `:`: src/:foo
"));
}

#[test]
fn repackage_on_source_change() {
    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
        "#)
        .file("src/main.rs", r#"
            fn main() { println!("hello"); }
        "#);

    assert_that(p.baler_process("package"),
                execs().with_status(0));

    // Add another source file
    let mut file = File::create(p.root().join("src").join("foo.rs")).unwrap_or_else(|e| {
        panic!("could not create file {}: {}", p.root().join("src/foo.rs").display(), e)
    });

    file.write_all(br#"
        fn main() { println!("foo"); }
    "#).unwrap();
    std::mem::drop(file);

    let mut pro = process(&baler_exe());
    pro.arg("package").cwd(p.root());

    // Check that baler rebuilds the tarball
    assert_that(pro, execs().with_status(0).with_stderr(&format!("\
[WARNING] [..]
See [..]
[PACKAGING] foo v0.0.1 ({dir})
[VERIFYING] foo v0.0.1 ({dir})
[COMPILING] foo v0.0.1 ({dir}[..])
[FINISHED] dev [unoptimized + debuginfo] target(s) in [..]
",
        dir = p.url())));

    // Check that the tarball contains the added file
    let f = File::open(&p.root().join("target/package/foo-0.0.1.crate")).unwrap();
    let mut rdr = GzDecoder::new(f).unwrap();
    let mut contents = Vec::new();
    rdr.read_to_end(&mut contents).unwrap();
    let mut ar = Archive::new(&contents[..]);
    let entries = ar.entries().unwrap();
    let entry_paths = entries.map(|entry| {
        entry.unwrap().path().unwrap().into_owned()
    }).collect::<Vec<PathBuf>>();
    assert_that(&entry_paths, contains(vec![PathBuf::from("foo-0.0.1/src/foo.rs")]));
}

#[test]
#[cfg(unix)]
fn broken_symlink() {
    use std::os::unix::fs;

    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
            license = "MIT"
            description = 'foo'
            documentation = 'foo'
            homepage = 'foo'
            repository = 'foo'
        "#)
        .file("src/main.rs", r#"
            fn main() { println!("hello"); }
        "#);
    p.build();
    t!(fs::symlink("nowhere", &p.root().join("src/foo.rs")));

    assert_that(p.baler("package").arg("-v"),
                execs().with_status(101)
                       .with_stderr_contains("\
error: failed to prepare local package for uploading

Caused by:
  failed to open for archiving: `[..]foo.rs`

Caused by:
  [..]
"));
}

#[test]
fn do_not_package_if_repository_is_dirty() {
    let p = project("foo");
    p.build();

    // Create a Git repository containing a minimal Rust project.
    git::repo(&paths::root().join("foo"))
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            license = "MIT"
            description = "foo"
            documentation = "foo"
            homepage = "foo"
            repository = "foo"
        "#)
        .file("src/main.rs", "fn main() {}")
        .build();

    // Modify Baler.toml without committing the change.
    p.change_file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            license = "MIT"
            description = "foo"
            documentation = "foo"
            homepage = "foo"
            repository = "foo"
            # change
    "#);

    assert_that(p.baler("package"),
                execs().with_status(101)
                       .with_stderr("\
error: 1 files in the working directory contain changes that were not yet \
committed into git:

Baler.toml

to proceed despite this, pass the `--allow-dirty` flag
"));
}

#[test]
fn generated_manifest() {
    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []
            exclude = ["*.txt"]
            license = "MIT"
            description = "foo"

            [project.metadata]
            foo = 'bar'

            [workspace]

            [dependencies]
            bar = { path = "bar", version = "0.1" }
        "#)
        .file("src/main.rs", "")
        .file("bar/Baler.toml", r#"
            [package]
            name = "bar"
            version = "0.1.0"
            authors = []
        "#)
        .file("bar/src/lib.rs", "");

    assert_that(p.baler_process("package").arg("--no-verify"),
                execs().with_status(0));

    let f = File::open(&p.root().join("target/package/foo-0.0.1.crate")).unwrap();
    let mut rdr = GzDecoder::new(f).unwrap();
    let mut contents = Vec::new();
    rdr.read_to_end(&mut contents).unwrap();
    let mut ar = Archive::new(&contents[..]);
    let mut entry = ar.entries().unwrap()
                        .map(|f| f.unwrap())
                        .find(|e| e.path().unwrap().ends_with("Baler.toml"))
                        .unwrap();
    let mut contents = String::new();
    entry.read_to_string(&mut contents).unwrap();
    assert_that(&contents[..], equal_to(
r#"# THIS FILE IS AUTOMATICALLY GENERATED BY CARGO
#
# When uploading crates to the registry Cargo will automatically
# "normalize" Baler.toml files for maximal compatibility
# with all versions of Cargo and also rewrite `path` dependencies
# to registry (e.g. crates.io) dependencies
#
# If you believe there's an error in this file please file an
# issue against the MultiscaleHN/baler repository. If you're
# editing this file be aware that the upstream Baler.toml
# will likely look very different (and much more reasonable)

[package]
name = "foo"
version = "0.0.1"
authors = []
exclude = ["*.txt"]
description = "foo"
license = "MIT"

[package.metadata]
foo = "bar"
[dependencies.bar]
version = "0.1"
"#));
}

#[test]
fn ignore_workspace_specifier() {
    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"

            authors = []

            [workspace]

            [dependencies]
            bar = { path = "bar", version = "0.1" }
        "#)
        .file("src/main.rs", "")
        .file("bar/Baler.toml", r#"
            [package]
            name = "bar"
            version = "0.1.0"
            authors = []
            workspace = ".."
        "#)
        .file("bar/src/lib.rs", "");
    p.build();

    assert_that(p.baler("package").arg("--no-verify").cwd(p.root().join("bar")),
                execs().with_status(0));

    let f = File::open(&p.root().join("target/package/bar-0.1.0.crate")).unwrap();
    let mut rdr = GzDecoder::new(f).unwrap();
    let mut contents = Vec::new();
    rdr.read_to_end(&mut contents).unwrap();
    let mut ar = Archive::new(&contents[..]);
    let mut entry = ar.entries().unwrap()
                        .map(|f| f.unwrap())
                        .find(|e| e.path().unwrap().ends_with("Baler.toml"))
                        .unwrap();
    let mut contents = String::new();
    entry.read_to_string(&mut contents).unwrap();
    assert_that(&contents[..], equal_to(
r#"# THIS FILE IS AUTOMATICALLY GENERATED BY CARGO
#
# When uploading crates to the registry Cargo will automatically
# "normalize" Baler.toml files for maximal compatibility
# with all versions of Cargo and also rewrite `path` dependencies
# to registry (e.g. crates.io) dependencies
#
# If you believe there's an error in this file please file an
# issue against the MultiscaleHN/baler repository. If you're
# editing this file be aware that the upstream Baler.toml
# will likely look very different (and much more reasonable)

[package]
name = "bar"
version = "0.1.0"
authors = []
"#));
}

#[test]
fn package_two_kinds_of_deps() {
    Package::new("other", "1.0.0").publish();
    Package::new("other1", "1.0.0").publish();
    let p = project("foo")
        .file("Baler.toml", r#"
            [project]
            name = "foo"
            version = "0.0.1"
            authors = []

            [dependencies]
            other = "1.0"
            other1 = { version = "1.0" }
        "#)
        .file("src/main.rs", "");

    assert_that(p.baler_process("package").arg("--no-verify"),
                execs().with_status(0));
}
