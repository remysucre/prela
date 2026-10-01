
WITH UserVoteStats AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        COUNT(DISTINCT P.Id) AS PostCount
    FROM Users U
    LEFT JOIN Votes V ON U.Id = V.UserId
    LEFT JOIN Posts P ON V.PostId = P.Id
    WHERE U.Reputation > 1000
    GROUP BY U.Id, U.DisplayName
),
TopUsers AS (
    SELECT 
        UserId,
        DisplayName,
        UpVotes,
        DownVotes,
        PostCount,
        ROW_NUMBER() OVER (ORDER BY UpVotes - DownVotes DESC, UserId) AS UserRank
    FROM UserVoteStats
),
PopularTags AS (
    SELECT 
        Tags.TagName,
        COUNT(P.Id) AS Popularity
    FROM Tags 
    JOIN Posts P ON P.Tags LIKE CONCAT('%', Tags.TagName, '%')
    GROUP BY Tags.TagName
    ORDER BY Popularity DESC
    LIMIT 5
)
SELECT 
    T.TagName,
    U.DisplayName,
    U.UpVotes,
    U.DownVotes,
    U.PostCount,
    (U.UpVotes - U.DownVotes) AS VoteBalance
FROM TopUsers U
JOIN PopularTags T ON U.PostCount > 5
WHERE U.UserRank <= 10
ORDER BY T.Popularity DESC, VoteBalance DESC;
