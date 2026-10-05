WITH PostCounts AS (
    SELECT 
        PostTypeId,
        COUNT(*) AS PostCount 
    FROM 
        Posts 
    GROUP BY 
        PostTypeId
), 
UserReputations AS (
    SELECT 
        Reputation,
        COUNT(*) AS UserCount 
    FROM 
        Users 
    GROUP BY 
        Reputation
), 
AveragePostViewCount AS (
    SELECT 
        AVG(ViewCount) AS AvgViewCount 
    FROM 
        Posts 
), 
PopularTags AS (
    SELECT 
        TagName, 
        SUM(Count) AS TotalCount 
    FROM 
        Tags 
    GROUP BY 
        TagName 
    ORDER BY 
        TotalCount DESC 
    LIMIT 5
)
SELECT 
    (SELECT COUNT(*) FROM Users) AS TotalUsers,
    (SELECT COUNT(*) FROM Posts) AS TotalPosts,
    (SELECT COUNT(*) FROM Comments) AS TotalComments,
    (SELECT COUNT(*) FROM Votes) AS TotalVotes,
    (SELECT COUNT(*) FROM Badges) AS TotalBadges,
    (SELECT AVG(PostCount) FROM PostCounts) AS AvgPostsPerType,
    (SELECT AVG(UserCount) FROM UserReputations) AS AvgUsersPerReputation,
    (SELECT AvgViewCount FROM AveragePostViewCount) AS AvgViewCount,
    (SELECT STRING_AGG(TagName, ', ' ORDER BY TotalCount DESC) FROM PopularTags) AS TopTags
;