WITH UserEngagement AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        COUNT(DISTINCT P.Id) AS PostCount,
        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS Comments
    FROM 
        Users U
    LEFT JOIN 
        Posts P ON U.Id = P.OwnerUserId
    LEFT JOIN 
        Votes V ON P.Id = V.PostId
    LEFT JOIN 
        Comments C ON P.Id = C.PostId
    WHERE 
        U.Reputation > 1000
    GROUP BY 
        U.Id, U.DisplayName
),
TopUsers AS (
    SELECT 
        UserId,
        DisplayName,
        PostCount,
        Questions,
        Answers,
        UpVotes,
        DownVotes,
        Comments,
        DENSE_RANK() OVER (ORDER BY PostCount DESC) AS EngagementRank
    FROM 
        UserEngagement
)
SELECT 
    U.DisplayName,
    U.PostCount,
    U.Questions,
    U.Answers,
    U.UpVotes,
    U.DownVotes,
    U.Comments,
    PT.Name AS PostType,
    COALESCE(T.TagName, 'No Tags') AS TagName,
    CASE 
        WHEN U.PostCount > 50 THEN 'High Engagement'
        WHEN U.PostCount BETWEEN 20 AND 50 THEN 'Moderate Engagement'
        ELSE 'Low Engagement'
    END AS EngagementLevel
FROM 
    TopUsers U
LEFT JOIN 
    Posts P ON U.UserId = P.OwnerUserId
LEFT JOIN 
    PostTypes PT ON P.PostTypeId = PT.Id
LEFT JOIN 
    LATERAL (
        SELECT 
            string_agg(T.TagName, ', ' ORDER BY T.TagName) AS TagName
        FROM 
            Tags T
        WHERE 
            P.Tags LIKE '%' || T.TagName || '%'
    ) T ON TRUE
WHERE 
    U.EngagementRank <= 10
ORDER BY 
    U.EngagementRank;
