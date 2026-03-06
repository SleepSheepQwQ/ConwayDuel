#!/usr/bin/env sh

##############################################################################
##
##  Gradle start up script for UN*X
##
##############################################################################

# Attempt to set APP_HOME
# Resolve links: $0 may be a link
PRG="$0"
# Need this for daisy-chained symlinks.
while [ -h "$PRG" ] ; do
    ls=`ls -ld "$PRG"`
    link=`expr "$ls" : '.*-> \(.*\)$'`
    if expr "$link" : '/.*' > /dev/null; then
        PRG="$link"
    else
        PRG=`dirname "$PRG"`"/$link"
    fi
done
SAVED="`pwd`"
cd "`dirname "$PRG"`/" >/dev/null
APP_HOME="`pwd -P`"
cd "$SAVED" >/dev/null

APP_NAME="Gradle"
APP_BASE_NAME=`basename "$0"`

# Add default JVM options here. You can also use JAVA_OPTS and GRADLE_OPTS to pass JVM options to this script.
DEFAULT_JVM_OPTS='"-Xmx64m" "-Xms64m"'

# Use the maximum available 64bit JVM.
if [ -z "$JAVA_HOME" ]; then
  JAVA_EXE="java"
  which java >/dev/null 2>&1 || { echo >&2 "ERROR: JAVA_HOME is not set and no 'java' command could be found in your PATH."; exit 1; }
else
  JAVA_EXE="$JAVA_HOME/bin/java"
  test -x "$JAVA_EXE" || JAVA_EXE="$JAVA_HOME/jre/bin/java"
  test -x "$JAVA_EXE" || { echo >&2 "ERROR: JAVA_HOME is set to an invalid directory: $JAVA_HOME"; exit 1; }
fi

CLASSPATH=$APP_HOME/gradle/wrapper/gradle-wrapper.jar

# Collect all arguments for the java command, following the shell quoting and substitution rules
eval set -- $DEFAULT_JVM_OPTS $JAVA_OPTS $GRADLE_OPTS ""-Dorg.gradle.appname=$APP_BASE_NAME"" -classpath ""$CLASSPATH"" org.gradle.wrapper.GradleWrapperMain "$@"

exec "$JAVA_EXE" "$@"
